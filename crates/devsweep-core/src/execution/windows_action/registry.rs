//! Isolated registry subject stored in an offline hive file.
//!
//! The hive is created and opened with `offreg.dll`. It is not loaded into
//! HKCU, HKLM, or any other mounted host key, and it is not a caller-supplied
//! registry path.

use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::Path,
    ptr,
};

const HIVE_MAGIC: &[u8] = b"regf";
const VALUE_KIND_BINARY: u32 = 3;
const FIXTURE_KEY: &str = "Fixture";
const FIXTURE_VALUE: &str = "Subject";
const MAX_VALUE_BYTES: usize = 1024 * 1024;

pub(crate) fn create_hive(path: &Path, value: &[u8]) -> io::Result<()> {
    if value.len() > MAX_VALUE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "registry fixture value is too large",
        ));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    #[cfg(windows)]
    {
        write_new_hive(path, value)
    }
    #[cfg(not(windows))]
    {
        let _ = (path, value);
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "offline registry fixtures require Windows",
        ))
    }
}

pub(crate) fn read_value(path: &Path) -> io::Result<Vec<u8>> {
    #[cfg(windows)]
    {
        read_hive_value(path)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "offline registry fixtures require Windows",
        ))
    }
}

pub(crate) fn hive_is_offline_file(path: &Path) -> io::Result<bool> {
    let mut header = [0u8; 4];
    let file = fs::File::open(path)?;
    use std::io::Read;
    let mut file = file;
    file.read_exact(&mut header)?;
    Ok(header == HIVE_MAGIC)
}

/// Rewrites the fixed fixture value and keeps the existing hive file object.
#[cfg(test)]
pub(crate) fn replace_value_in_place(path: &Path, value: &[u8]) -> io::Result<()> {
    if value.len() > MAX_VALUE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "registry fixture value is too large",
        ));
    }
    #[cfg(windows)]
    {
        let temp = temp_hive_path(path);
        let _ = fs::remove_file(&temp);
        rewrite_hive_to(&temp, path, value)?;
        let bytes = fs::read(&temp)?;
        write_in_place(path, &bytes)?;
        let _ = fs::remove_file(&temp);
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = (path, value);
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "offline registry fixtures require Windows",
        ))
    }
}

pub(crate) fn write_in_place(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).truncate(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

#[cfg(windows)]
fn write_new_hive(path: &Path, value: &[u8]) -> io::Result<()> {
    let _ = fs::remove_file(path);
    let hive = create_memory_hive(value)?;
    let wide = wide_path(path)?;
    let status = unsafe { or_save_hive(hive.0, wide.as_ptr(), 6, 1) };
    let status = if status == 0 {
        0
    } else {
        unsafe { or_save_hive(hive.0, wide.as_ptr(), 5, 1) }
    };
    drop(hive);
    if status != 0 {
        return Err(os_status("ORSaveHive", status));
    }
    if !hive_is_offline_file(path)? {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "saved hive is missing the regf header",
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn read_hive_value(path: &Path) -> io::Result<Vec<u8>> {
    let hive = open_hive(path)?;
    let bytes = get_value(hive.0)?;
    drop(hive);
    Ok(bytes)
}

#[cfg(all(windows, test))]
fn rewrite_hive_to(temp: &Path, source: &Path, value: &[u8]) -> io::Result<()> {
    let hive = open_hive(source)?;
    let key_name = wide_str(FIXTURE_KEY);
    let mut key = ptr::null_mut();
    let opened = unsafe { or_open_key(hive.0, key_name.as_ptr(), &raw mut key) };
    if opened != 0 {
        return Err(os_status("OROpenKey", opened));
    }
    let key = Key(key);
    set_binary(key.0, value)?;
    drop(key);
    let wide = wide_path(temp)?;
    let saved = unsafe { or_save_hive(hive.0, wide.as_ptr(), 6, 1) };
    let saved = if saved == 0 {
        0
    } else {
        unsafe { or_save_hive(hive.0, wide.as_ptr(), 5, 1) }
    };
    drop(hive);
    if saved != 0 {
        return Err(os_status("or_save_hive", saved));
    }
    Ok(())
}

#[cfg(windows)]
fn create_memory_hive(value: &[u8]) -> io::Result<Hive> {
    let mut hive = ptr::null_mut();
    let created = unsafe { or_create_hive(&raw mut hive) };
    if created != 0 {
        return Err(os_status("ORCreateHive", created));
    }
    let hive = Hive(hive);
    let key_name = wide_str(FIXTURE_KEY);
    let mut key = ptr::null_mut();
    let created = unsafe {
        or_create_key(
            hive.0,
            key_name.as_ptr(),
            ptr::null_mut(),
            0,
            ptr::null_mut(),
            &raw mut key,
            ptr::null_mut(),
        )
    };
    if created != 0 {
        return Err(os_status("ORCreateKey", created));
    }
    let key = Key(key);
    set_binary(key.0, value)?;
    drop(key);
    Ok(hive)
}

#[cfg(windows)]
fn set_binary(key: *mut core::ffi::c_void, value: &[u8]) -> io::Result<()> {
    let name = wide_str(FIXTURE_VALUE);
    let len = u32::try_from(value.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "registry value is too large"))?;
    let data = if value.is_empty() {
        ptr::null()
    } else {
        value.as_ptr()
    };
    let status = unsafe { or_set_value(key, name.as_ptr(), VALUE_KIND_BINARY, data, len) };
    if status != 0 {
        return Err(os_status("ORSetValue", status));
    }
    Ok(())
}

#[cfg(windows)]
fn open_hive(path: &Path) -> io::Result<Hive> {
    let wide = wide_path(path)?;
    let mut hive = ptr::null_mut();
    let status = unsafe { or_open_hive(wide.as_ptr(), &raw mut hive) };
    if status != 0 {
        return Err(os_status("OROpenHive", status));
    }
    Ok(Hive(hive))
}

#[cfg(windows)]
fn get_value(hive: *mut core::ffi::c_void) -> io::Result<Vec<u8>> {
    let subkey = wide_str(FIXTURE_KEY);
    let name = wide_str(FIXTURE_VALUE);
    let mut kind = 0u32;
    let mut length = u32::try_from(MAX_VALUE_BYTES).unwrap_or(u32::MAX);
    let mut buffer = vec![0u8; MAX_VALUE_BYTES];
    let status = unsafe {
        or_get_value(
            hive,
            subkey.as_ptr(),
            name.as_ptr(),
            &raw mut kind,
            buffer.as_mut_ptr(),
            &raw mut length,
        )
    };
    if status != 0 {
        return Err(os_status("ORGetValue", status));
    }
    if kind != VALUE_KIND_BINARY {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "fixture registry value is not REG_BINARY",
        ));
    }
    let length = usize::try_from(length).unwrap_or(0);
    if length > buffer.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "fixture registry value length is invalid",
        ));
    }
    buffer.truncate(length);
    Ok(buffer)
}

#[cfg(all(windows, test))]
fn temp_hive_path(path: &Path) -> std::path::PathBuf {
    let mut name = path
        .file_name()
        .map(std::ffi::OsStr::to_os_string)
        .unwrap_or_default();
    name.push(".tmp");
    path.with_file_name(name)
}

#[cfg(windows)]
fn wide_str(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
fn wide_path(path: &Path) -> io::Result<Vec<u16>> {
    use std::os::windows::ffi::OsStrExt;
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    if wide.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "path contains an interior nul",
        ));
    }
    wide.push(0);
    Ok(wide)
}

#[cfg(windows)]
fn os_status(name: &str, status: u32) -> io::Error {
    io::Error::other(format!("{name} failed with status {status}"))
}

#[cfg(windows)]
struct Hive(*mut core::ffi::c_void);

#[cfg(windows)]
impl Drop for Hive {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { or_close_hive(self.0) };
            self.0 = ptr::null_mut();
        }
    }
}

#[cfg(windows)]
struct Key(*mut core::ffi::c_void);

#[cfg(windows)]
impl Drop for Key {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { or_close_key(self.0) };
            self.0 = ptr::null_mut();
        }
    }
}

#[cfg(windows)]
struct Offreg {
    create_hive: unsafe extern "system" fn(*mut *mut core::ffi::c_void) -> u32,
    open_hive: unsafe extern "system" fn(*const u16, *mut *mut core::ffi::c_void) -> u32,
    save_hive: unsafe extern "system" fn(*mut core::ffi::c_void, *const u16, u32, u32) -> u32,
    close_hive: unsafe extern "system" fn(*mut core::ffi::c_void) -> u32,
    create_key: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *const u16,
        *mut u16,
        u32,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
        *mut u32,
    ) -> u32,
    close_key: unsafe extern "system" fn(*mut core::ffi::c_void) -> u32,
    #[cfg_attr(not(test), allow(dead_code))]
    open_key: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *const u16,
        *mut *mut core::ffi::c_void,
    ) -> u32,
    set_value:
        unsafe extern "system" fn(*mut core::ffi::c_void, *const u16, u32, *const u8, u32) -> u32,
    get_value: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *const u16,
        *const u16,
        *mut u32,
        *mut u8,
        *mut u32,
    ) -> u32,
}

#[cfg(windows)]
fn offreg() -> io::Result<&'static Offreg> {
    use std::sync::OnceLock;
    static CELL: OnceLock<Result<Offreg, String>> = OnceLock::new();
    match CELL.get_or_init(|| load_offreg().map_err(|error| error.to_string())) {
        Ok(api) => Ok(api),
        Err(message) => Err(io::Error::other(message.clone())),
    }
}

#[cfg(windows)]
fn load_offreg() -> io::Result<Offreg> {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn LoadLibraryW(name: *const u16) -> *mut core::ffi::c_void;
    }
    let name = wide_str("offreg.dll");
    let module = unsafe { LoadLibraryW(name.as_ptr()) };
    if module.is_null() {
        return Err(io::Error::last_os_error());
    }
    Ok(Offreg {
        create_hive: proc(module, "ORCreateHive")?,
        open_hive: proc(module, "OROpenHive")?,
        save_hive: proc(module, "ORSaveHive")?,
        close_hive: proc(module, "ORCloseHive")?,
        create_key: proc(module, "ORCreateKey")?,
        close_key: proc(module, "ORCloseKey")?,
        open_key: proc(module, "OROpenKey")?,
        set_value: proc(module, "ORSetValue")?,
        get_value: proc(module, "ORGetValue")?,
    })
}

#[cfg(windows)]
fn proc<T>(module: *mut core::ffi::c_void, name: &str) -> io::Result<T> {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetProcAddress(
            module: *mut core::ffi::c_void,
            name: *const i8,
        ) -> *mut core::ffi::c_void;
    }
    let mut symbol = Vec::with_capacity(name.len() + 1);
    symbol.extend(name.bytes().map(i8::try_from).filter_map(Result::ok));
    symbol.push(0);
    let address = unsafe { GetProcAddress(module, symbol.as_ptr()) };
    if address.is_null() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("offreg.dll is missing {name}"),
        ));
    }
    // SAFETY: `address` is the exported function and `T` is that function's type.
    Ok(unsafe { std::mem::transmute_copy(&address) })
}

#[cfg(windows)]
unsafe fn or_create_hive(result: *mut *mut core::ffi::c_void) -> u32 {
    match offreg() {
        Ok(api) => unsafe { (api.create_hive)(result) },
        Err(_) => 126,
    }
}

#[cfg(windows)]
unsafe fn or_open_hive(path: *const u16, result: *mut *mut core::ffi::c_void) -> u32 {
    match offreg() {
        Ok(api) => unsafe { (api.open_hive)(path, result) },
        Err(_) => 126,
    }
}

#[cfg(windows)]
unsafe fn or_save_hive(
    hive: *mut core::ffi::c_void,
    path: *const u16,
    major: u32,
    minor: u32,
) -> u32 {
    match offreg() {
        Ok(api) => unsafe { (api.save_hive)(hive, path, major, minor) },
        Err(_) => 126,
    }
}

#[cfg(windows)]
unsafe fn or_close_hive(hive: *mut core::ffi::c_void) -> u32 {
    match offreg() {
        Ok(api) => unsafe { (api.close_hive)(hive) },
        Err(_) => 126,
    }
}

#[cfg(windows)]
unsafe fn or_create_key(
    hive: *mut core::ffi::c_void,
    subkey: *const u16,
    class: *mut u16,
    options: u32,
    security: *mut core::ffi::c_void,
    result: *mut *mut core::ffi::c_void,
    disposition: *mut u32,
) -> u32 {
    match offreg() {
        Ok(api) => unsafe {
            (api.create_key)(hive, subkey, class, options, security, result, disposition)
        },
        Err(_) => 126,
    }
}

#[cfg(windows)]
unsafe fn or_close_key(key: *mut core::ffi::c_void) -> u32 {
    match offreg() {
        Ok(api) => unsafe { (api.close_key)(key) },
        Err(_) => 126,
    }
}

#[cfg(all(windows, test))]
unsafe fn or_open_key(
    hive: *mut core::ffi::c_void,
    subkey: *const u16,
    result: *mut *mut core::ffi::c_void,
) -> u32 {
    match offreg() {
        Ok(api) => unsafe { (api.open_key)(hive, subkey, result) },
        Err(_) => 126,
    }
}

#[cfg(windows)]
unsafe fn or_set_value(
    key: *mut core::ffi::c_void,
    name: *const u16,
    kind: u32,
    data: *const u8,
    len: u32,
) -> u32 {
    match offreg() {
        Ok(api) => unsafe { (api.set_value)(key, name, kind, data, len) },
        Err(_) => 126,
    }
}

#[cfg(windows)]
unsafe fn or_get_value(
    hive: *mut core::ffi::c_void,
    subkey: *const u16,
    name: *const u16,
    kind: *mut u32,
    data: *mut u8,
    len: *mut u32,
) -> u32 {
    match offreg() {
        Ok(api) => unsafe { (api.get_value)(hive, subkey, name, kind, data, len) },
        Err(_) => 126,
    }
}

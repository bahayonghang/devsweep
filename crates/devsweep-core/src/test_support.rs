//! Shared helpers for unit-test fixtures.

/// Creates a temporary directory for fixtures that ownership and no-follow
/// safety checks must accept.
///
/// The no-follow checks reject any root reached through a symlink. macOS
/// links `/var` to `/private/var`, so on Unix the system temp directory is
/// canonicalized first. On Windows the temp directory is expanded to its long
/// form without a verbatim `\\?\` prefix, because `TEMP` often holds an 8.3
/// short name (`RUNNER~1`) while canonicalized scan results use the long name,
/// and new fixture objects are made owned by the current user (see
/// [`own_new_objects_as_current_user`]).
pub(crate) fn tempdir() -> std::io::Result<tempfile::TempDir> {
    #[cfg(unix)]
    {
        tempfile::tempdir_in(std::env::temp_dir().canonicalize()?)
    }
    #[cfg(windows)]
    {
        own_new_objects_as_current_user();
        tempfile::tempdir_in(long_temp_dir())
    }
    #[cfg(not(any(unix, windows)))]
    {
        tempfile::tempdir()
    }
}

/// The system temp directory with every component in its long form.
///
/// Canonicalization yields `\\?\C:\...`; only that plain drive form is
/// unwrapped. Anything else (UNC, failure) falls back to the original path.
#[cfg(windows)]
fn long_temp_dir() -> std::path::PathBuf {
    let base = std::env::temp_dir();
    std::fs::canonicalize(&base)
        .ok()
        .and_then(|path| {
            let text = path.to_str()?.strip_prefix(r"\\?\")?;
            let bytes = text.as_bytes();
            (bytes.len() >= 3
                && bytes[0].is_ascii_alphabetic()
                && bytes[1] == b':'
                && bytes[2] == b'\\')
                .then(|| std::path::PathBuf::from(text))
        })
        .unwrap_or(base)
}

/// Makes objects this test process creates from now on owned by its user SID.
///
/// An elevated administrator token defaults new objects to the
/// `BUILTIN\Administrators` owner, as on GitHub's Windows runners. Temp
/// eligibility requires the owner to equal the token user, so fixtures created
/// there are correctly refused as not proven owned. Setting the token's
/// default owner to the user SID (always permitted for one's own token) makes
/// an elevated run match a standard one. Only this test process's token
/// changes; product code is untouched. Call it before creating fixtures, and
/// in spawned child test processes before they create files.
#[cfg(windows)]
pub(crate) fn own_new_objects_as_current_user() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        use std::{mem::size_of, ptr};
        use windows_sys::Win32::{
            Foundation::CloseHandle,
            Security::{
                GetTokenInformation, SetTokenInformation, TOKEN_ADJUST_DEFAULT, TOKEN_OWNER,
                TOKEN_QUERY, TOKEN_USER, TokenOwner, TokenUser,
            },
            System::Threading::{GetCurrentProcess, OpenProcessToken},
        };
        let mut token = ptr::null_mut();
        // SAFETY: the process pseudo-handle is always valid and `token` is a
        // writable out-pointer.
        let opened = unsafe {
            OpenProcessToken(
                GetCurrentProcess(),
                TOKEN_QUERY | TOKEN_ADJUST_DEFAULT,
                &mut token,
            )
        };
        assert_ne!(
            opened,
            0,
            "open process token: {}",
            std::io::Error::last_os_error()
        );
        let mut length = 0;
        // SAFETY: a null buffer with zero length only asks for the byte count.
        unsafe {
            GetTokenInformation(token, TokenUser, ptr::null_mut(), 0, &mut length);
        }
        assert!(length as usize >= size_of::<TOKEN_USER>() && length <= 65_536);
        let mut user = vec![0usize; (length as usize).div_ceil(size_of::<usize>())];
        // SAFETY: `user` is usize-aligned and holds at least `length` bytes.
        let queried = unsafe {
            GetTokenInformation(
                token,
                TokenUser,
                user.as_mut_ptr().cast(),
                length,
                &mut length,
            )
        };
        assert_ne!(
            queried,
            0,
            "query token user: {}",
            std::io::Error::last_os_error()
        );
        let owner = TOKEN_OWNER {
            // SAFETY: the query succeeded, so the buffer starts with a TOKEN_USER
            // whose SID points into `user`, which outlives this call.
            Owner: unsafe { (*user.as_ptr().cast::<TOKEN_USER>()).User.Sid },
        };
        // SAFETY: `owner` is a live TOKEN_OWNER and the length is its size.
        let set = unsafe {
            SetTokenInformation(
                token,
                TokenOwner,
                ptr::from_ref(&owner).cast(),
                size_of::<TOKEN_OWNER>() as u32,
            )
        };
        let error = std::io::Error::last_os_error();
        // SAFETY: `token` was opened above and is closed exactly once.
        unsafe {
            CloseHandle(token);
        }
        assert_ne!(set, 0, "set token default owner: {error}");
    });
}

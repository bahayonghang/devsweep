use std::{
    env,
    path::{Path, PathBuf},
};

pub(crate) fn resolve_executable(program: &str) -> Option<PathBuf> {
    let path = Path::new(program);
    if path.components().count() > 1 && path.is_file() {
        return Some(path.to_path_buf());
    }
    for directory in env::split_paths(&env::var_os("PATH")?) {
        #[cfg(windows)]
        if path.extension().is_none() {
            let extensions =
                env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string());
            for extension in extensions.split(';').filter(|value| !value.is_empty()) {
                let candidate = directory.join(format!("{program}{extension}"));
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
        let candidate = directory.join(program);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

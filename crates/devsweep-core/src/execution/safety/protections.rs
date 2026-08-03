use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::filesystem::{normalize_path_for_compare, paths_equal};

#[cfg(not(windows))]
use super::resolve_home_dir;

/// Versioned user protection list stored in OS app-data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserProtectionList {
    path: PathBuf,
    entries: Vec<PathBuf>,
    /// Test-only in-memory mode never touches the real config path.
    in_memory: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct UserProtectionDocument {
    version: u32,
    paths: Vec<String>,
}

const USER_PROTECTION_VERSION: u32 = 1;

impl UserProtectionList {
    /// Returns the platform-specific protection-list path.
    pub(crate) fn config_path() -> Result<PathBuf> {
        Ok(app_data_dir()?.join("protected-paths.json"))
    }

    /// Loads the persisted protection list or returns an empty list when absent.
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self {
                path,
                entries: Vec::new(),
                in_memory: false,
            });
        }
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read protection list {}", path.display()))?;
        let doc: UserProtectionDocument = serde_json::from_str(&raw)
            .with_context(|| format!("failed to parse protection list {}", path.display()))?;
        if doc.version != USER_PROTECTION_VERSION {
            bail!(
                "unsupported protection list version {} in {}",
                doc.version,
                path.display()
            );
        }
        let entries = doc.paths.into_iter().map(PathBuf::from).collect::<Vec<_>>();
        Ok(Self {
            path,
            entries,
            in_memory: false,
        })
    }

    #[cfg(test)]
    pub(super) fn load_from_path(path: PathBuf) -> Result<Self> {
        if !path.exists() {
            return Ok(Self {
                path,
                entries: Vec::new(),
                in_memory: false,
            });
        }
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read protection list {}", path.display()))?;
        let doc: UserProtectionDocument = serde_json::from_str(&raw)
            .with_context(|| format!("failed to parse protection list {}", path.display()))?;
        if doc.version != USER_PROTECTION_VERSION {
            bail!(
                "unsupported protection list version {} in {}",
                doc.version,
                path.display()
            );
        }
        Ok(Self {
            path,
            entries: doc.paths.into_iter().map(PathBuf::from).collect(),
            in_memory: false,
        })
    }

    /// In-memory empty list for unit tests that must not touch app-data.
    pub(crate) fn empty_in_memory_for_tests_only() -> Self {
        Self {
            path: PathBuf::from("memory://protected-paths.json"),
            entries: Vec::new(),
            in_memory: true,
        }
    }

    /// Returns all normalized protected paths.
    pub(crate) fn paths(&self) -> &[PathBuf] {
        &self.entries
    }

    /// Adds an existing path and persists the updated list.
    pub fn add(&mut self, path: &Path) -> Result<()> {
        if !path.exists() {
            bail!("protect add requires an existing path: {}", path.display());
        }
        let canonical = path
            .canonicalize()
            .with_context(|| format!("failed to canonicalize {}", path.display()))?;
        let canonical = normalize_path_for_compare(&canonical);
        if self
            .entries
            .iter()
            .any(|existing| paths_equal(existing, &canonical))
        {
            return Ok(());
        }
        self.entries.push(canonical);
        self.persist()
    }

    /// Removes a path and reports whether an entry changed.
    pub fn remove(&mut self, path: &Path) -> Result<bool> {
        let candidate = if path.exists() {
            normalize_path_for_compare(
                &path
                    .canonicalize()
                    .with_context(|| format!("failed to canonicalize {}", path.display()))?,
            )
        } else if path.is_absolute() {
            normalize_path_for_compare(path)
        } else {
            normalize_path_for_compare(&env::current_dir()?.join(path))
        };

        let before = self.entries.len();
        self.entries
            .retain(|existing| !paths_equal(existing, &candidate));
        let removed = self.entries.len() != before;
        if removed {
            self.persist()?;
        }
        Ok(removed)
    }

    /// Returns all normalized protected paths for display.
    pub fn list(&self) -> &[PathBuf] {
        &self.entries
    }

    fn persist(&self) -> Result<()> {
        if self.in_memory {
            return Ok(());
        }
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create protection list directory {}",
                    parent.display()
                )
            })?;
        }
        let doc = UserProtectionDocument {
            version: USER_PROTECTION_VERSION,
            paths: self
                .entries
                .iter()
                .map(|path| path.display().to_string())
                .collect(),
        };
        let payload =
            serde_json::to_vec_pretty(&doc).context("failed to encode protection list")?;
        let parent = self
            .path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let temp_path = parent.join(format!(".protected-paths.{}.tmp", std::process::id()));
        {
            let mut file = fs::File::create(&temp_path).with_context(|| {
                format!(
                    "failed to create temporary protection list {}",
                    temp_path.display()
                )
            })?;
            file.write_all(&payload)
                .and_then(|_| file.write_all(b"\n"))
                .and_then(|_| file.sync_all())
                .with_context(|| {
                    format!(
                        "failed to write temporary protection list for {}",
                        self.path.display()
                    )
                })?;
        }
        fs::rename(&temp_path, &self.path).with_context(|| {
            format!(
                "failed to atomically replace protection list {}",
                self.path.display()
            )
        })?;
        Ok(())
    }
}

fn app_data_dir() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        let base = env::var_os("APPDATA")
            .map(PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("APPDATA is not set"))?;
        Ok(base.join("devsweep"))
    }
    #[cfg(target_os = "macos")]
    {
        let home = resolve_home_dir().ok_or_else(|| anyhow::anyhow!("HOME is not set"))?;
        Ok(home.join("Library/Application Support/devsweep"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(xdg) = env::var_os("XDG_CONFIG_HOME") {
            return Ok(PathBuf::from(xdg).join("devsweep"));
        }
        let home = resolve_home_dir().ok_or_else(|| anyhow::anyhow!("HOME is not set"))?;
        Ok(home.join(".config/devsweep"))
    }
    #[cfg(not(any(windows, unix)))]
    {
        bail!("unsupported platform for protection list storage")
    }
}

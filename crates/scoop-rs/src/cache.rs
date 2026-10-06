use std::path::{Path, PathBuf};

use crate::constant::REGEX_CACHE_FILE;
use crate::error::{Error, Fallible};

/// Scoop cache file representation
#[derive(Clone, Debug)]
pub struct CacheFile {
    path: PathBuf,
}

impl CacheFile {
    pub fn from(path: PathBuf) -> Fallible<CacheFile> {
        let text = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        match REGEX_CACHE_FILE.is_match(text) {
            false => Err(Error::InvalidCacheFile { path }),
            true => Ok(CacheFile { path }),
        }
    }

    /// Get path of this cache file
    #[inline]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get file name of this cache file
    #[inline]
    pub fn file_name(&self) -> &str {
        self.path.file_name().unwrap().to_str().unwrap()
    }

    /// Get package name of this cache file
    #[inline]
    pub fn package_name(&self) -> &str {
        self.file_name().split_once('#').map(|s| s.0).unwrap()
    }

    /// Get version of this cache file
    #[inline]
    pub fn version(&self) -> &str {
        self.file_name().splitn(3, '#').collect::<Vec<_>>()[1]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_cache_filenames() {
        // Garbage names must error, never panic (file_name/package_name/
        // version accessors are only safe because from() validates).
        assert!(CacheFile::from(PathBuf::from("README.md")).is_err());
        assert!(CacheFile::from(PathBuf::from("nohashes")).is_err());

        let valid = CacheFile::from(PathBuf::from("app#1.0#abc1234.zip")).unwrap();
        assert_eq!(valid.package_name(), "app");
        assert_eq!(valid.version(), "1.0");
    }
}

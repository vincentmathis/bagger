//! SQLite cache for bucket manifests.
//!
//! Parsing every bucket manifest from disk on each operation is one of the
//! slowest parts of a Scoop workflow (thousands of small JSON files). When
//! the `use_sqlite_cache` config is enabled, raw manifest JSON is cached in
//! a SQLite database keyed by `(bucket, name)` together with the file's
//! mtime and size, so unchanged manifests can be served without touching
//! the bucket files. Entries are refreshed lazily: any mtime/size mismatch
//! triggers a re-parse and cache update.

use std::path::Path;
use std::sync::Mutex;
use std::time::UNIX_EPOCH;

use rusqlite::{params, OptionalExtension};

use crate::error::{Error, Fallible};

/// mtime+size fingerprint used to invalidate stale cache entries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileFingerprint {
    /// File modification time as seconds since the Unix epoch.
    pub mtime_secs: i64,
    /// File size in bytes.
    pub size: u64,
}

impl FileFingerprint {
    /// Compute the fingerprint of the file at `path`.
    ///
    /// # Errors
    ///
    /// I/O errors will be returned if the metadata cannot be read.
    pub fn of(path: &Path) -> Fallible<Self> {
        let meta = std::fs::metadata(path)?;
        let mtime_secs = meta
            .modified()?
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        Ok(Self {
            mtime_secs,
            size: meta.len(),
        })
    }
}

/// SQLite-backed manifest cache.
///
/// The inner connection is behind a [`Mutex`] so the cache can be shared
/// across the rayon worker threads used by package queries.
pub struct ManifestCache {
    conn: Mutex<rusqlite::Connection>,
}

impl ManifestCache {
    /// Open (creating if needed) the cache database in `cache_dir`.
    ///
    /// # Errors
    ///
    /// I/O errors will be returned if the directory or database file cannot
    /// be created. SQLite errors will be returned if the schema cannot be
    /// initialized.
    pub fn open(cache_dir: &Path) -> Fallible<Self> {
        std::fs::create_dir_all(cache_dir)?;
        let conn = rusqlite::Connection::open(cache_dir.join("manifests.db"))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS manifests (
                bucket TEXT NOT NULL,
                name TEXT NOT NULL,
                mtime INTEGER NOT NULL,
                size INTEGER NOT NULL,
                json BLOB NOT NULL,
                PRIMARY KEY (bucket, name)
            );",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Look up a cached manifest, returning its raw JSON on a fingerprint hit.
    ///
    /// Returns `Ok(None)` on a cache miss or when the cached fingerprint no
    /// longer matches the file.
    pub fn get(&self, bucket: &str, name: &str, fp: FileFingerprint) -> Fallible<Option<Vec<u8>>> {
        let conn = self.conn.lock().map_err(|e| Error::Custom(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT mtime, size, json FROM manifests WHERE bucket = ?1 AND name = ?2")?;
        let row: Option<(i64, i64, Vec<u8>)> = stmt
            .query_row([bucket, name], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .optional()?;

        Ok(row.and_then(|(mtime, size, json)| {
            let size = u64::try_from(size).unwrap_or(u64::MAX);
            if mtime == fp.mtime_secs && size == fp.size {
                Some(json)
            } else {
                None
            }
        }))
    }

    /// Store raw manifest JSON under `(bucket, name)`.
    pub fn put(&self, bucket: &str, name: &str, fp: FileFingerprint, json: &[u8]) -> Fallible<()> {
        let conn = self.conn.lock().map_err(|e| Error::Custom(e.to_string()))?;
        let size = i64::try_from(fp.size).unwrap_or(i64::MAX);
        conn.execute(
            "INSERT OR REPLACE INTO manifests (bucket, name, mtime, size, json)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![bucket, name, fp.mtime_secs, size, json],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "bagger-test-manifest-cache-{}-{}",
            tag,
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn roundtrip_hit_and_invalidation() {
        let dir = unique_dir("roundtrip");
        let cache = ManifestCache::open(&dir).unwrap();

        // Miss on empty cache.
        let fp = FileFingerprint {
            mtime_secs: 123,
            size: 3,
        };
        assert!(cache.get("main", "7zip", fp).unwrap().is_none());

        // Hit after put.
        cache.put("main", "7zip", fp, b"{}").unwrap();
        assert_eq!(cache.get("main", "7zip", fp).unwrap(), Some(b"{}".to_vec()));

        // Stale fingerprint misses.
        let stale = FileFingerprint {
            mtime_secs: 124,
            size: 3,
        };
        assert!(cache.get("main", "7zip", stale).unwrap().is_none());

        // Overwrite updates the entry.
        cache.put("main", "7zip", stale, b"[]").unwrap();
        assert_eq!(
            cache.get("main", "7zip", stale).unwrap(),
            Some(b"[]".to_vec())
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn fingerprint_of_real_file() {
        let dir = unique_dir("fingerprint");
        let file = dir.join("app.json");
        std::fs::write(&file, b"{}").unwrap();

        let fp = FileFingerprint::of(&file).unwrap();
        assert_eq!(fp.size, 2);

        assert!(FileFingerprint::of(&dir.join("missing.json")).is_err());

        std::fs::remove_dir_all(&dir).ok();
    }
}

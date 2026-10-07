//! Optional heavy-machinery progress strings.
//!
//! Bagger's default output stays plain and script-compatible. Set
//! `BAGGER_FLAVOR=heavy` to swap progress lines for deadpan industrial
//! equivalents ("Hauling..." instead of "Downloading packages...").
//!
//! Nothing else changes: same events, same exit codes, same machine-readable
//! output. Scripts should leave the variable unset.

/// Whether the heavy-machinery flavor is enabled.
pub fn heavy() -> bool {
    std::env::var("BAGGER_FLAVOR")
        .map(|v| v.eq_ignore_ascii_case("heavy"))
        .unwrap_or(false)
}

/// Look up a progress line by key.
///
/// Keys: `resolve`, `sizing`, `download`, `download_complete`, `integrity`,
/// `verifying`, `verified`, `cached`, `buckets`.
pub fn progress(key: &str) -> &str {
    if heavy() {
        match key {
            "resolve" => "Surveying the pit...",
            "sizing" => "Weighing the load...",
            "download" => "Hauling...",
            "download_complete" => "Hauled.",
            "integrity" => "Assaying the ore...",
            "verifying" => "Assaying the ore...",
            "verified" => "Assayed.",
            "cached" => "Nothing to haul — already stockpiled.",
            "buckets" => "Rotating the wheel...",
            _ => key,
        }
    } else {
        match key {
            "resolve" => "Resolving packages...",
            "sizing" => "Calculating download size...",
            "download" => "Downloading packages...",
            "download_complete" => "Download complete.",
            "integrity" => "Checking package integrity...",
            "verifying" => "Verifying hashes...",
            "verified" => "Hash verification complete.",
            "cached" => "Nothing to download, all cached.",
            "buckets" => "Updating buckets",
            _ => key,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Serialize env mutation; Rust runs tests on multiple threads.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn default_output_is_plain() {
        let _lock = ENV_LOCK.lock().unwrap();
        std::env::remove_var("BAGGER_FLAVOR");
        assert!(!heavy());
        assert_eq!(progress("resolve"), "Resolving packages...");
        assert_eq!(progress("sizing"), "Calculating download size...");
        assert_eq!(progress("download"), "Downloading packages...");
        assert_eq!(progress("download_complete"), "Download complete.");
        assert_eq!(progress("integrity"), "Checking package integrity...");
        assert_eq!(progress("verifying"), "Verifying hashes...");
        assert_eq!(progress("verified"), "Hash verification complete.");
        assert_eq!(progress("cached"), "Nothing to download, all cached.");
        assert_eq!(progress("buckets"), "Updating buckets");
    }

    #[test]
    fn heavy_flavor_swaps_strings() {
        let _lock = ENV_LOCK.lock().unwrap();
        std::env::set_var("BAGGER_FLAVOR", "Heavy");
        assert!(heavy());
        assert_eq!(progress("resolve"), "Surveying the pit...");
        assert_eq!(progress("sizing"), "Weighing the load...");
        assert_eq!(progress("download"), "Hauling...");
        assert_eq!(progress("download_complete"), "Hauled.");
        assert_eq!(progress("integrity"), "Assaying the ore...");
        assert_eq!(progress("verifying"), "Assaying the ore...");
        assert_eq!(progress("verified"), "Assayed.");
        assert_eq!(progress("cached"), "Nothing to haul — already stockpiled.");
        assert_eq!(progress("buckets"), "Rotating the wheel...");
        std::env::remove_var("BAGGER_FLAVOR");
    }
}

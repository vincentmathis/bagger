//! Runtime CPU architecture selection.
//!
//! Manifest field resolution (`url`, `hash`, `bin`, …) is architecture
//! specific. By default the host architecture is used, but callers (such as
//! `bagger install --arch 32bit`, or the `SCOOP_ARCH` environment variable)
//! may override it process-wide via [`set_override`].

use std::sync::RwLock;

use crate::error::{Error, Fallible};

/// CPU architectures selectable for manifest resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arch {
    /// 32-bit x86 (`32bit` manifests).
    Ia32,
    /// 64-bit x86_64 (`64bit` manifests).
    Amd64,
    /// 64-bit ARM (`arm64` manifests).
    Aarch64,
}

impl Arch {
    /// Canonical Scoop manifest name of this architecture.
    pub fn as_str(self) -> &'static str {
        match self {
            Arch::Ia32 => "32bit",
            Arch::Amd64 => "64bit",
            Arch::Aarch64 => "arm64",
        }
    }

    /// Parse a Scoop architecture name (`32bit`, `64bit`, `arm64`).
    ///
    /// Common aliases (`x86`/`ia32`, `x64`/`x86_64`/`amd64`, `aarch64`) are
    /// accepted case-insensitively.
    pub fn parse(name: &str) -> Fallible<Arch> {
        match name.to_ascii_lowercase().as_str() {
            "32bit" | "ia32" | "x86" => Ok(Arch::Ia32),
            "64bit" | "amd64" | "x64" | "x86_64" => Ok(Arch::Amd64),
            "arm64" | "aarch64" => Ok(Arch::Aarch64),
            _ => Err(Error::Custom(format!(
                "invalid architecture '{name}' (expected 32bit, 64bit or arm64)"
            ))),
        }
    }

    /// The default architecture for the compiling host.
    fn host() -> Arch {
        if cfg!(target_arch = "x86") {
            Arch::Ia32
        } else if cfg!(target_arch = "aarch64") {
            Arch::Aarch64
        } else {
            Arch::Amd64
        }
    }
}

static OVERRIDE: RwLock<Option<Arch>> = RwLock::new(None);

/// Override the architecture used for manifest resolution.
///
/// An invalid name returns an error and leaves any previous override
/// untouched.
pub fn set_override(name: &str) -> Fallible<()> {
    let arch = Arch::parse(name)?;
    if let Ok(mut guard) = OVERRIDE.write() {
        *guard = Some(arch);
    }
    Ok(())
}

/// Clear a previously set architecture override.
pub fn clear_override() {
    if let Ok(mut guard) = OVERRIDE.write() {
        *guard = None;
    }
}

/// The currently effective override, if any.
pub fn override_arch() -> Option<Arch> {
    OVERRIDE.read().ok().and_then(|guard| *guard)
}

/// The architecture used for manifest resolution.
///
/// This is the override when set, otherwise the host architecture.
pub fn selected() -> Arch {
    override_arch().unwrap_or_else(Arch::host)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_names() {
        assert_eq!(Arch::parse("32bit").unwrap(), Arch::Ia32);
        assert_eq!(Arch::parse("64bit").unwrap(), Arch::Amd64);
        assert_eq!(Arch::parse("arm64").unwrap(), Arch::Aarch64);
        assert_eq!(Arch::parse("x86_64").unwrap(), Arch::Amd64);
        assert!(Arch::parse("mips").is_err());
    }

    #[test]
    fn override_roundtrip() {
        clear_override();
        assert_eq!(override_arch(), None);
        assert_eq!(selected(), Arch::host());

        set_override("32bit").unwrap();
        assert_eq!(override_arch(), Some(Arch::Ia32));
        assert_eq!(selected(), Arch::Ia32);

        // Invalid names leave the previous override untouched.
        assert!(set_override("bogus").is_err());
        assert_eq!(selected(), Arch::Ia32);

        clear_override();
        assert_eq!(selected(), Arch::host());
    }

    /// The override must steer manifest arch selection end to end.
    ///
    /// Kept in this module (rather than scattered across manifest tests) so
    /// exactly one test mutates the process-wide override.
    #[test]
    fn override_steers_manifest_selection() {
        use crate::package::manifest::Manifest;

        let manifest = Manifest::parse_bytes(
            br#"{
                "version": "1.0",
                "homepage": "https://example.com",
                "license": "MIT",
                "architecture": {
                    "32bit": {
                        "url": "https://example.com/x86.zip",
                        "hash": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                    },
                    "64bit": {
                        "url": "https://example.com/x64.zip",
                        "hash": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                    }
                }
            }"#,
            std::path::Path::new("arch-test.json"),
        )
        .expect("fixture manifest should parse");

        clear_override();
        // Host here is x86_64, so the 64bit section wins by default.
        assert_eq!(manifest.url(), vec!["https://example.com/x64.zip"]);

        set_override("32bit").unwrap();
        assert_eq!(manifest.url(), vec!["https://example.com/x86.zip"]);

        clear_override();
        assert_eq!(manifest.url(), vec!["https://example.com/x64.zip"]);
    }
}

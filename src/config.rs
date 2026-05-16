use std::fs;

use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use serde::Deserialize;

const CONFIG_FILENAME: &str = "fwmap.toml";

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct FwmapConfig {
    pub package: Option<String>,
    pub target: Option<String>,
    pub profile: Option<String>,
    pub features: Option<Vec<String>>,
    pub no_default_features: Option<bool>,
    pub budget: Option<Utf8PathBuf>,
    pub top_symbols: Option<usize>,
    pub map_path: Option<Utf8PathBuf>,
}

impl FwmapConfig {
    /// Walk up from `start` looking for `fwmap.toml`. Returns default on miss.
    pub fn discover(start: &Utf8Path) -> Result<Self> {
        let mut dir: Option<&Utf8Path> = Some(start);
        while let Some(current) = dir {
            let candidate = current.join(CONFIG_FILENAME);
            if candidate.is_file() {
                let raw = fs::read_to_string(&candidate)
                    .with_context(|| format!("failed to read {candidate}"))?;
                let config: FwmapConfig = toml::from_str(&raw)
                    .with_context(|| format!("failed to parse {candidate}"))?;
                return Ok(config);
            }
            dir = current.parent();
        }
        Ok(Self::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn utf8(path: &std::path::Path) -> Utf8PathBuf {
        Utf8PathBuf::from_path_buf(path.to_owned()).expect("non-utf8 test tempdir")
    }

    #[test]
    fn discover_returns_default_when_no_config_present() {
        let dir = TempDir::new().unwrap();
        let config = FwmapConfig::discover(&utf8(dir.path())).unwrap();
        assert!(config.package.is_none());
        assert!(config.features.is_none());
    }

    #[test]
    fn discover_walks_up_to_find_config() {
        let root = TempDir::new().unwrap();
        let nested = root.path().join("a").join("b").join("c");
        fs::create_dir_all(&nested).unwrap();

        let config_path = root.path().join("fwmap.toml");
        fs::write(
            &config_path,
            r#"
package = "my-firmware"
target = "thumbv7em-none-eabihf"
features = ["foo", "bar"]
no-default-features = true
"#,
        )
        .unwrap();

        let config = FwmapConfig::discover(&utf8(&nested)).unwrap();
        assert_eq!(config.package.as_deref(), Some("my-firmware"));
        assert_eq!(config.target.as_deref(), Some("thumbv7em-none-eabihf"));
        assert_eq!(
            config.features.as_deref(),
            Some(["foo".to_owned(), "bar".to_owned()].as_slice())
        );
        assert_eq!(config.no_default_features, Some(true));
    }

    #[test]
    fn discover_rejects_unknown_field() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("fwmap.toml");
        fs::write(&config_path, "pkg = \"oops\"\n").unwrap();
        let err = FwmapConfig::discover(&utf8(dir.path())).unwrap_err();
        let chain: Vec<String> = err.chain().map(|e| e.to_string()).collect();
        assert!(
            chain.iter().any(|m| m.contains("unknown field")),
            "expected unknown-field error, got: {chain:?}"
        );
    }
}

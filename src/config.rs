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

use anyhow::anyhow;

use crate::cli::{FirmwareReportArgs, FirmwareValidateArgs};
use crate::workspace::WorkspaceLayout;

const DEFAULT_PROFILE: &str = "release";
const DEFAULT_TOP_SYMBOLS: usize = 20;
const DEFAULT_MAP_FILENAME: &str = "firmware.map";

#[derive(Debug, Clone)]
pub struct ResolvedReport {
    pub package: String,
    pub target: String,
    pub profile: String,
    pub features: Vec<String>,
    pub no_default_features: bool,
    pub map_path: Utf8PathBuf,
    pub output: Option<Utf8PathBuf>,
    pub allow_build_failure: bool,
    pub top_symbols: usize,
}

#[derive(Debug, Clone)]
pub struct ResolvedValidate {
    pub report: ResolvedReport,
    pub budget: Utf8PathBuf,
}

impl ResolvedReport {
    pub fn merge(
        cli: FirmwareReportArgs,
        config: &FwmapConfig,
        layout: &WorkspaceLayout,
    ) -> Result<Self> {
        let package = cli
            .build
            .package
            .or_else(|| config.package.clone())
            .ok_or_else(|| {
                anyhow!("no `package` set: add `package = \"...\"` to fwmap.toml or pass --package")
            })?;
        let target = cli
            .build
            .target
            .or_else(|| config.target.clone())
            .ok_or_else(|| {
                anyhow!("no `target` set: add `target = \"...\"` to fwmap.toml or pass --target")
            })?;
        let profile = cli
            .build
            .profile
            .or_else(|| config.profile.clone())
            .unwrap_or_else(|| DEFAULT_PROFILE.to_owned());

        let (features, no_default_features) = resolve_features(
            cli.build.features,
            cli.build.no_default_features,
            config,
        );

        let map_path = cli
            .build
            .map_path
            .or_else(|| config.map_path.clone())
            .unwrap_or_else(|| layout.target_dir.join(DEFAULT_MAP_FILENAME));

        let top_symbols = cli
            .top_symbols
            .or(config.top_symbols)
            .unwrap_or(DEFAULT_TOP_SYMBOLS);

        Ok(Self {
            package,
            target,
            profile,
            features,
            no_default_features,
            map_path,
            output: cli.output,
            allow_build_failure: cli.allow_build_failure,
            top_symbols,
        })
    }
}

impl ResolvedValidate {
    pub fn merge(
        cli: FirmwareValidateArgs,
        config: &FwmapConfig,
        layout: &WorkspaceLayout,
    ) -> Result<Self> {
        let budget = cli
            .budget
            .or_else(|| config.budget.clone())
            .ok_or_else(|| {
                anyhow!("no `budget` set: add `budget = \"...\"` to fwmap.toml or pass --budget")
            })?;
        let report_args = FirmwareReportArgs {
            build: cli.build,
            output: None,
            allow_build_failure: true,
            top_symbols: None,
        };
        let report = ResolvedReport::merge(report_args, config, layout)?;
        Ok(Self { report, budget })
    }
}

fn resolve_features(
    cli_features: Vec<String>,
    cli_no_default: bool,
    config: &FwmapConfig,
) -> (Vec<String>, bool) {
    if !cli_features.is_empty() {
        return (cli_features, cli_no_default);
    }
    let features = config.features.clone().unwrap_or_default();
    let no_default = cli_no_default || config.no_default_features.unwrap_or(false);
    (features, no_default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::cli::{FirmwareBuildArgs, FirmwareReportArgs, FirmwareValidateArgs};
    use crate::workspace::WorkspaceLayout;

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
        let chain: Vec<String> = err.chain().map(ToString::to_string).collect();
        assert!(
            chain.iter().any(|m| m.contains("unknown field")),
            "expected unknown-field error, got: {chain:?}"
        );
    }

    fn dummy_layout() -> WorkspaceLayout {
        WorkspaceLayout {
            root: Utf8PathBuf::from("/repo"),
            target_dir: Utf8PathBuf::from("/repo/target"),
        }
    }

    fn empty_build_args() -> FirmwareBuildArgs {
        FirmwareBuildArgs {
            package: None,
            target: None,
            profile: None,
            features: Vec::new(),
            no_default_features: false,
            map_path: None,
        }
    }

    #[test]
    fn resolve_report_uses_config_when_cli_empty() {
        let cli = FirmwareReportArgs {
            build: empty_build_args(),
            output: None,
            allow_build_failure: false,
            top_symbols: None,
        };
        let config = FwmapConfig {
            package: Some("from-config".to_owned()),
            target: Some("thumbv7em-none-eabihf".to_owned()),
            features: Some(vec!["a".to_owned()]),
            no_default_features: Some(true),
            top_symbols: Some(10),
            ..Default::default()
        };
        let resolved = ResolvedReport::merge(cli, &config, &dummy_layout()).unwrap();
        assert_eq!(resolved.package, "from-config");
        assert_eq!(resolved.target, "thumbv7em-none-eabihf");
        assert_eq!(resolved.profile, "release");
        assert_eq!(resolved.features, vec!["a"]);
        assert!(resolved.no_default_features);
        assert_eq!(resolved.top_symbols, 10);
        assert_eq!(resolved.map_path, Utf8PathBuf::from("/repo/target/firmware.map"));
    }

    #[test]
    fn resolve_report_cli_overrides_config() {
        let cli = FirmwareReportArgs {
            build: FirmwareBuildArgs {
                package: Some("from-cli".to_owned()),
                target: Some("x86_64-pc-windows-msvc".to_owned()),
                features: vec!["x".to_owned(), "y".to_owned()],
                no_default_features: true,
                ..empty_build_args()
            },
            output: None,
            allow_build_failure: false,
            top_symbols: Some(5),
        };
        let config = FwmapConfig {
            package: Some("from-config".to_owned()),
            target: Some("thumbv7em-none-eabihf".to_owned()),
            features: Some(vec!["from-config".to_owned()]),
            top_symbols: Some(100),
            ..Default::default()
        };
        let resolved = ResolvedReport::merge(cli, &config, &dummy_layout()).unwrap();
        assert_eq!(resolved.package, "from-cli");
        assert_eq!(resolved.target, "x86_64-pc-windows-msvc");
        assert_eq!(resolved.features, vec!["x", "y"]);
        assert!(resolved.no_default_features);
        assert_eq!(resolved.top_symbols, 5);
    }

    #[test]
    fn resolve_report_errors_when_package_missing() {
        let cli = FirmwareReportArgs {
            build: empty_build_args(),
            output: None,
            allow_build_failure: false,
            top_symbols: None,
        };
        let config = FwmapConfig::default();
        let err = ResolvedReport::merge(cli, &config, &dummy_layout()).unwrap_err();
        assert!(err.to_string().contains("package"));
    }

    #[test]
    fn resolve_report_errors_when_target_missing() {
        let cli = FirmwareReportArgs {
            build: FirmwareBuildArgs {
                package: Some("demo".to_owned()),
                ..empty_build_args()
            },
            output: None,
            allow_build_failure: false,
            top_symbols: None,
        };
        let config = FwmapConfig::default();
        let err = ResolvedReport::merge(cli, &config, &dummy_layout()).unwrap_err();
        assert!(err.to_string().contains("target"));
    }

    #[test]
    fn resolve_validate_errors_when_budget_missing() {
        let cli = FirmwareValidateArgs {
            build: FirmwareBuildArgs {
                package: Some("demo".to_owned()),
                target: Some("thumbv7em-none-eabihf".to_owned()),
                ..empty_build_args()
            },
            budget: None,
        };
        let config = FwmapConfig::default();
        let err = ResolvedValidate::merge(cli, &config, &dummy_layout()).unwrap_err();
        assert!(err.to_string().contains("budget"));
    }
}

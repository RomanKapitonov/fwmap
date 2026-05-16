use camino::Utf8PathBuf;
use clap::{ArgAction, Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "cargo fwmap",
    about = "Firmware memory reporting and budget validation"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Build firmware and emit a target memory report.
    FirmwareReport(FirmwareReportArgs),
    /// Build firmware and validate target memory against a JSON budget.
    FirmwareValidate(FirmwareValidateArgs),
}

#[derive(Debug, Clone, Args)]
pub struct FirmwareBuildArgs {
    /// Cargo package to build. Falls back to fwmap.toml `package`.
    #[arg(long)]
    pub package: Option<String>,
    /// Cargo target triple. Falls back to fwmap.toml `target`.
    #[arg(long)]
    pub target: Option<String>,
    /// Cargo profile. Defaults to "release".
    #[arg(long)]
    pub profile: Option<String>,
    /// Cargo features (comma-separated, repeatable). Falls back to fwmap.toml `features`.
    #[arg(long, value_delimiter = ',', action = ArgAction::Append)]
    pub features: Vec<String>,
    /// Disable the firmware crate's default feature set.
    #[arg(long)]
    pub no_default_features: bool,
    /// Override path to the linker `.map` file.
    #[arg(long)]
    pub map_path: Option<Utf8PathBuf>,
}

#[derive(Debug, Clone, Args)]
pub struct FirmwareReportArgs {
    #[command(flatten)]
    pub build: FirmwareBuildArgs,
    /// Optional output path for the JSON report. Stdout is used when omitted.
    #[arg(long)]
    pub output: Option<Utf8PathBuf>,
    /// Continue and emit a report even when the firmware build fails.
    #[arg(long)]
    pub allow_build_failure: bool,
    /// Number of top symbols to include in the report.
    #[arg(long)]
    pub top_symbols: Option<usize>,
}

#[derive(Debug, Clone, Args)]
pub struct FirmwareValidateArgs {
    #[command(flatten)]
    pub build: FirmwareBuildArgs,
    /// Budget file to validate against. Falls back to fwmap.toml `budget`.
    #[arg(long)]
    pub budget: Option<Utf8PathBuf>,
}

impl Cli {
    pub fn command_or_default(self) -> Command {
        self.command
            .unwrap_or_else(|| Command::FirmwareReport(FirmwareReportArgs::default_for_no_subcommand()))
    }
}

impl FirmwareReportArgs {
    fn default_for_no_subcommand() -> Self {
        Self {
            build: FirmwareBuildArgs {
                package: None,
                target: None,
                profile: None,
                features: Vec::new(),
                no_default_features: false,
                map_path: None,
            },
            output: None,
            allow_build_failure: true,
            top_symbols: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{Cli, Command};

    #[test]
    fn no_subcommand_defaults_to_firmware_report_with_allow_build_failure() {
        let cli = Cli::try_parse_from(["fwmap"]).expect("no-subcommand CLI should parse");
        let Command::FirmwareReport(args) = cli.command_or_default() else {
            panic!("expected firmware-report");
        };
        assert!(args.allow_build_failure);
    }

    #[test]
    fn firmware_report_args_flatten_exposes_shared_fields() {
        let cli = Cli::try_parse_from([
            "fwmap",
            "firmware-report",
            "--package", "demo",
            "--target", "thumbv7em-none-eabihf",
            "--features", "a,b",
        ])
        .expect("flattened CLI should parse");
        let Command::FirmwareReport(args) = cli.command_or_default() else {
            panic!("expected firmware-report");
        };
        assert_eq!(args.build.package.as_deref(), Some("demo"));
        assert_eq!(args.build.target.as_deref(), Some("thumbv7em-none-eabihf"));
        assert_eq!(args.build.features, vec!["a".to_owned(), "b".to_owned()]);
    }

    #[test]
    fn firmware_validate_args_share_build_args_with_report() {
        let cli = Cli::try_parse_from([
            "fwmap",
            "firmware-validate",
            "--package", "demo",
            "--target", "thumbv7em-none-eabihf",
            "--budget", "budget.json",
        ])
        .expect("flattened validate CLI should parse");
        let Command::FirmwareValidate(args) = cli.command_or_default() else {
            panic!("expected firmware-validate");
        };
        assert_eq!(args.build.package.as_deref(), Some("demo"));
        assert_eq!(args.budget.as_deref(), Some(camino::Utf8Path::new("budget.json")));
    }
}

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
pub struct FirmwareReportArgs {
    /// Cargo package to build.
    #[arg(long, default_value = "effects-mcu")]
    pub package: String,
    /// Optional output path for the JSON report. Stdout is used when omitted.
    #[arg(long)]
    pub output: Option<Utf8PathBuf>,
    /// Cargo target triple used for the firmware build.
    #[arg(long, default_value = "thumbv7em-none-eabihf")]
    pub target: String,
    /// Cargo profile used for the firmware build.
    #[arg(long, default_value = "release")]
    pub profile: String,
    /// Cargo features to enable for the firmware build.
    #[arg(
        long,
        value_delimiter = ',',
        action = ArgAction::Append
    )]
    pub features: Vec<String>,
    /// Disable the firmware crate's default feature set.
    #[arg(long)]
    pub no_default_features: bool,
    /// Continue and emit a report even when the firmware build fails.
    #[arg(long)]
    pub allow_build_failure: bool,
}

#[derive(Debug, Clone, Args)]
pub struct FirmwareValidateArgs {
    /// Budget file to validate against.
    #[arg(long, default_value = "support/memory/firmware-memory-budget.json")]
    pub budget: Utf8PathBuf,
    /// Cargo package to build.
    #[arg(long, default_value = "effects-mcu")]
    pub package: String,
    /// Cargo target triple used for the firmware build.
    #[arg(long, default_value = "thumbv7em-none-eabihf")]
    pub target: String,
    /// Cargo profile used for the firmware build.
    #[arg(long, default_value = "release")]
    pub profile: String,
    /// Cargo features to enable for the firmware build.
    #[arg(
        long,
        value_delimiter = ',',
        action = ArgAction::Append
    )]
    pub features: Vec<String>,
    /// Disable the firmware crate's default feature set.
    #[arg(long)]
    pub no_default_features: bool,
}

fn default_firmware_features() -> Vec<String> {
    vec!["greenfield-build".to_owned(), "capture".to_owned()]
}

impl Cli {
    pub fn command_or_default(self) -> Command {
        self.command
            .unwrap_or_else(|| Command::FirmwareReport(FirmwareReportArgs::default_for_fwmap()))
    }
}

impl FirmwareReportArgs {
    fn default_for_fwmap() -> Self {
        Self {
            package: "effects-mcu".to_owned(),
            output: None,
            target: "thumbv7em-none-eabihf".to_owned(),
            profile: "release".to_owned(),
            features: Vec::new(),
            no_default_features: false,
            allow_build_failure: true,
        }
    }
}

impl FirmwareReportArgs {
    pub fn effective_features(&self) -> Vec<String> {
        if self.features.is_empty() {
            default_firmware_features()
        } else {
            self.features.clone()
        }
    }

    pub fn effective_no_default_features(&self) -> bool {
        self.no_default_features || self.features.is_empty()
    }
}

impl FirmwareValidateArgs {
    pub fn effective_features(&self) -> Vec<String> {
        if self.features.is_empty() {
            default_firmware_features()
        } else {
            self.features.clone()
        }
    }

    pub fn effective_no_default_features(&self) -> bool {
        self.no_default_features || self.features.is_empty()
    }

    pub fn as_report_args(&self, allow_build_failure: bool) -> FirmwareReportArgs {
        FirmwareReportArgs {
            package: self.package.clone(),
            output: None,
            target: self.target.clone(),
            profile: self.profile.clone(),
            features: self.effective_features(),
            no_default_features: self.effective_no_default_features(),
            allow_build_failure,
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{Cli, Command};

    #[test]
    fn no_subcommand_defaults_to_firmware_report() {
        let cli = Cli::try_parse_from(["fwmap"]).expect("no-subcommand CLI should parse");

        assert!(matches!(
            cli.command_or_default(),
            Command::FirmwareReport(_)
        ));
    }

    #[test]
    fn default_firmware_features_select_greenfield_capture_build() {
        let cli = Cli::try_parse_from(["fwmap"]).expect("no-subcommand CLI should parse");
        let Command::FirmwareReport(args) = cli.command_or_default() else {
            panic!("default command should be firmware-report");
        };

        assert_eq!(
            args.effective_features(),
            ["greenfield-build".to_owned(), "capture".to_owned()]
        );
        assert!(args.effective_no_default_features());
        assert!(args.allow_build_failure);
    }
}

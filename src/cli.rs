use camino::Utf8PathBuf;
use clap::{ArgAction, Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "cargo xtask",
    about = "Firmware memory reporting and budget validation"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
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
}

fn default_firmware_features() -> Vec<String> {
    vec![
        "effect-runtime-experimental-dag".to_owned(),
        "ccmram".to_owned(),
    ]
}

impl FirmwareReportArgs {
    pub fn effective_features(&self) -> Vec<String> {
        if self.features.is_empty() {
            default_firmware_features()
        } else {
            self.features.clone()
        }
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

    pub fn as_report_args(&self, allow_build_failure: bool) -> FirmwareReportArgs {
        FirmwareReportArgs {
            package: self.package.clone(),
            output: None,
            target: self.target.clone(),
            profile: self.profile.clone(),
            features: self.effective_features(),
            allow_build_failure,
        }
    }
}

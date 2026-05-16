mod budget;
mod cli;
mod config;
mod elf;
mod firmware;
mod map;
mod report;
mod validate;
mod workspace;

use std::process::ExitCode;

use anyhow::Result;
use camino::Utf8PathBuf;
use clap::Parser;

use budget::load_firmware_memory_budget;
use cli::{Cli, Command};
use config::{FwmapConfig, ResolvedReport, ResolvedValidate};
use firmware::{collect_firmware_report, write_firmware_report};
use validate::validate_firmware_budget;
use workspace::WorkspaceLayout;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let layout = WorkspaceLayout::discover()?;
    let cwd = Utf8PathBuf::from_path_buf(std::env::current_dir()?)
        .map_err(|p| anyhow::anyhow!("cwd is not valid UTF-8: {}", p.display()))?;
    let config = FwmapConfig::discover(&cwd)?;

    match cli.command_or_default() {
        Command::FirmwareReport(args) => {
            let resolved = ResolvedReport::merge(args, &config, &layout)?;
            let report = collect_firmware_report(&resolved, &layout)?;
            write_firmware_report(resolved.output.as_deref(), &report)
        }
        Command::FirmwareValidate(args) => {
            let resolved = ResolvedValidate::merge(args, &config, &layout)?;
            let report = collect_firmware_report(&resolved.report, &layout)?;
            let budget = load_firmware_memory_budget(&resolved.budget)?;
            let errors = validate_firmware_budget(&report, &budget);

            if errors.is_empty() {
                println!("firmware memory validation passed");
                Ok(())
            } else {
                anyhow::bail!(errors.join("\n"));
            }
        }
    }
}

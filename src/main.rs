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
use clap::Parser;

use budget::load_firmware_memory_budget;
use cli::{Cli, Command};
use firmware::{collect_firmware_report, write_firmware_report};
use validate::validate_firmware_budget;

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

    match cli.command_or_default() {
        Command::FirmwareReport(args) => {
            let report = collect_firmware_report(&args)?;
            write_firmware_report(args.output.as_deref(), &report)
        }
        Command::FirmwareValidate(args) => {
            let report = collect_firmware_report(&args.as_report_args(true))?;
            let budget = load_firmware_memory_budget(&args.budget)?;
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

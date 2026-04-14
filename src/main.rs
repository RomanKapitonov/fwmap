mod budget;
mod cli;
mod elf;
mod firmware;
mod map;
mod report;
mod size_probe;
mod validate;

use std::process::ExitCode;

use budget::load_firmware_memory_budget;
use clap::Parser;
use cli::{Cli, Command};
use firmware::{collect_firmware_report, write_firmware_report};
use size_probe::{
    collect_size_probe_report, collect_size_probe_report_for_validation,
    validate_size_probe_budget, write_json_report,
};
use validate::validate_firmware_budget;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();

    match cli.command {
        Command::SizeProbeReport(args) => {
            let report = collect_size_probe_report(&args)?;
            write_json_report(args.output.as_deref(), &report)
        }
        Command::SizeProbeValidate(args) => {
            let report = collect_size_probe_report_for_validation(&args)?;
            let budget = budget::load_size_probe_budget(&args.budget)?;
            let errors = validate_size_probe_budget(&report, &budget);

            if errors.is_empty() {
                println!("size-probe validation passed");
                Ok(())
            } else {
                Err(errors.join("\n"))
            }
        }
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
                Err(errors.join("\n"))
            }
        }
    }
}

use std::{collections::BTreeMap, process::Command};

use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use cargo_metadata::{Message, TargetKind};
use regex::Regex;

use crate::{
    cli::FirmwareReportArgs,
    elf::read_elf_sections,
    map::{read_map_sections, read_top_symbols},
    report::{FirmwareBuildReport, FirmwareLinkerReport, FirmwareMemoryReport},
};

const TOP_SYMBOL_COUNT: usize = 20;

pub fn collect_firmware_report(args: &FirmwareReportArgs) -> Result<FirmwareMemoryReport> {
    let build = run_firmware_build(args)?;
    let features = args.effective_features();
    let linker = parse_linker_summary(&build.diagnostics);
    let sections = collect_sections(build.elf_path.as_deref(), &build.map_path)?;
    let top_symbols = if build.map_path.is_file() {
        read_top_symbols(&build.map_path, TOP_SYMBOL_COUNT)?
    } else {
        Vec::new()
    };

    Ok(FirmwareMemoryReport {
        schema_version: 1,
        build: FirmwareBuildReport {
            target: args.target.clone(),
            profile: args.profile.clone(),
            features,
            succeeded: build.exit_code == 0,
            exit_code: build.exit_code,
            map_path: build.map_path.to_string(),
            elf_path: build.elf_path.map(|path| path.to_string()),
        },
        linker,
        sections,
        top_symbols,
    })
}

pub fn write_firmware_report(
    path: Option<&Utf8Path>,
    report: &FirmwareMemoryReport,
) -> Result<()> {
    let rendered = serde_json::to_string_pretty(report)
        .context("failed to serialize firmware report")?;

    if let Some(path) = path {
        std::fs::write(path, rendered).with_context(|| format!("failed to write {path}"))?;
    } else {
        println!("{rendered}");
    }

    Ok(())
}

struct FirmwareBuildCapture {
    exit_code: i32,
    diagnostics: String,
    map_path: Utf8PathBuf,
    elf_path: Option<Utf8PathBuf>,
}

fn run_firmware_build(args: &FirmwareReportArgs) -> Result<FirmwareBuildCapture> {
    let repo_root = repo_root();
    let mut command = Command::new("cargo");
    command
        .args(cargo_build_args(args, &repo_root))
        .current_dir(&repo_root);

    let output = command
        .output()
        .context("failed to run firmware build")?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let (elf_path, rendered_messages) = parse_cargo_messages(&stdout)?;

    let exit_code = output.status.code().unwrap_or(1);
    let mut diagnostics = stderr;
    if !rendered_messages.is_empty() {
        if !diagnostics.is_empty() {
            diagnostics.push('\n');
        }
        diagnostics.push_str(&rendered_messages.join("\n"));
    }

    if exit_code != 0 && !args.allow_build_failure {
        anyhow::bail!("firmware build failed with exit code {exit_code}\n{diagnostics}");
    }

    Ok(FirmwareBuildCapture {
        exit_code,
        diagnostics,
        map_path: repo_root.join("target/firmware.map"),
        elf_path,
    })
}

fn cargo_build_args(args: &FirmwareReportArgs, repo_root: &Utf8Path) -> Vec<String> {
    let features = args.effective_features();
    let manifest_path = Utf8PathBuf::from(format!("{}/Cargo.toml", args.package));
    let mut cargo_args = vec!["build".to_owned(), "--locked".to_owned()];

    if repo_root.join(&manifest_path).is_file() {
        cargo_args.push("--manifest-path".to_owned());
        cargo_args.push(manifest_path.to_string());
    } else {
        cargo_args.push("-p".to_owned());
        cargo_args.push(args.package.clone());
    }

    cargo_args.extend([
        "--target".to_owned(),
        args.target.clone(),
        "--message-format=json".to_owned(),
    ]);

    match args.profile.as_str() {
        "release" => {
            cargo_args.push("--release".to_owned());
        }
        "dev" => {}
        other => {
            cargo_args.push("--profile".to_owned());
            cargo_args.push(other.to_owned());
        }
    }

    if args.effective_no_default_features() {
        cargo_args.push("--no-default-features".to_owned());
    }
    if !features.is_empty() {
        cargo_args.push("--features".to_owned());
        cargo_args.push(features.join(","));
    }

    cargo_args
}

fn parse_cargo_messages(stdout: &str) -> Result<(Option<Utf8PathBuf>, Vec<String>)> {
    let mut elf_path = None;
    let mut rendered_messages = Vec::new();
    let cursor = std::io::Cursor::new(stdout.as_bytes());

    for message in Message::parse_stream(cursor) {
        let message = message.context("failed to parse cargo JSON message")?;
        match message {
            Message::CompilerArtifact(artifact) => {
                if artifact
                    .target
                    .kind
                    .iter()
                    .any(|kind| kind == &TargetKind::Bin)
                    && let Some(executable) = artifact.executable
                {
                    elf_path = Some(executable);
                }
            }
            Message::CompilerMessage(message) => {
                if let Some(rendered) = message.message.rendered
                    && !rendered.trim().is_empty()
                {
                    rendered_messages.push(rendered);
                }
            }
            _ => {}
        }
    }

    Ok((elf_path, rendered_messages))
}

fn parse_linker_summary(output: &str) -> FirmwareLinkerReport {
    let ram_overflow = Regex::new(
        r"section '\.bss' will not fit in region 'RAM': overflowed by (?P<bytes>\d+) bytes",
    )
    .expect("RAM overflow regex is valid");
    let uninit_overflow = Regex::new(
        r"section '\.uninit' will not fit in region 'RAM': overflowed by (?P<bytes>\d+) bytes",
    )
    .expect("uninit overflow regex is valid");
    let stack_placement = Regex::new(r"stack end address is not below stack start")
        .expect("stack placement regex is valid");

    FirmwareLinkerReport {
        ram_overflow_bytes: capture_bytes(&ram_overflow, output),
        uninit_overflow_bytes: capture_bytes(&uninit_overflow, output),
        stack_placement_failed: stack_placement.is_match(output),
    }
}

fn collect_sections(
    elf_path: Option<&Utf8Path>,
    map_path: &Utf8Path,
) -> Result<BTreeMap<String, u64>> {
    if let Some(elf_path) = elf_path
        && elf_path.is_file()
    {
        return read_elf_sections(elf_path);
    }

    if map_path.is_file() {
        return read_map_sections(map_path);
    }

    Ok(BTreeMap::new())
}

fn capture_bytes(regex: &Regex, text: &str) -> u64 {
    regex
        .captures(text)
        .and_then(|captures| captures.name("bytes"))
        .and_then(|capture| capture.as_str().parse().ok())
        .unwrap_or(0)
}

fn repo_root() -> Utf8PathBuf {
    Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("fwmap lives under the workspace root")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::{cargo_build_args, repo_root};
    use crate::cli::FirmwareReportArgs;

    #[test]
    fn default_firmware_build_uses_standalone_effects_manifest() {
        let args = FirmwareReportArgs {
            package: "effects-mcu".to_owned(),
            output: None,
            target: "thumbv7em-none-eabihf".to_owned(),
            profile: "release".to_owned(),
            features: Vec::new(),
            no_default_features: false,
            allow_build_failure: false,
        };

        let cargo_args = cargo_build_args(&args, &repo_root());

        assert_eq!(
            cargo_args,
            [
                "build",
                "--locked",
                "--manifest-path",
                "effects-mcu/Cargo.toml",
                "--target",
                "thumbv7em-none-eabihf",
                "--message-format=json",
                "--release",
                "--no-default-features",
                "--features",
                "greenfield-build,capture",
            ]
        );
    }
}

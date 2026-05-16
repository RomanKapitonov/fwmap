use std::process::Command;
use std::sync::LazyLock;

use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use cargo_metadata::{Message, TargetKind};
use regex::Regex;

static RAM_OVERFLOW_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"section '\.bss' will not fit in region 'RAM': overflowed by (?P<bytes>\d+) bytes",
    )
    .expect("RAM overflow regex is valid")
});
static UNINIT_OVERFLOW_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"section '\.uninit' will not fit in region 'RAM': overflowed by (?P<bytes>\d+) bytes",
    )
    .expect("uninit overflow regex is valid")
});
static STACK_PLACEMENT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"stack end address is not below stack start")
        .expect("stack placement regex is valid")
});

use crate::config::ResolvedReport;
use crate::elf::read_elf_sections;
use crate::map::{read_map_sections, read_top_symbols};
use crate::report::{
    FirmwareBuildReport, FirmwareLinkerReport, FirmwareMemoryReport, SectionReadout, SectionSource,
};
use crate::workspace::WorkspaceLayout;
use std::collections::BTreeMap;

pub fn collect_firmware_report(
    resolved: &ResolvedReport,
    layout: &WorkspaceLayout,
) -> Result<FirmwareMemoryReport> {
    let build = run_firmware_build(resolved, layout)?;
    let linker = parse_linker_summary(&build.diagnostics);
    let SectionReadout {
        sections,
        source: section_source,
    } = collect_sections(build.elf_path.as_deref(), &build.map_path)?;
    let top_symbols = if build.map_path.is_file() {
        read_top_symbols(&build.map_path, resolved.top_symbols)?
    } else {
        Vec::new()
    };

    Ok(FirmwareMemoryReport {
        build: FirmwareBuildReport {
            target: resolved.target.clone(),
            profile: resolved.profile.clone(),
            features: resolved.features.clone(),
            succeeded: build.exit_code == 0,
            exit_code: build.exit_code,
            map_path: build.map_path.to_string(),
            elf_path: build.elf_path.map(|path| path.to_string()),
            section_source,
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

fn run_firmware_build(
    resolved: &ResolvedReport,
    layout: &WorkspaceLayout,
) -> Result<FirmwareBuildCapture> {
    let mut command = Command::new("cargo");
    command
        .args(cargo_build_args(resolved, &layout.root))
        .current_dir(&layout.root);

    let output = command.output().context("failed to run firmware build")?;
    let stdout = String::from_utf8(output.stdout)
        .context("cargo build stdout was not valid UTF-8")?;
    let stderr: std::borrow::Cow<'_, str> = String::from_utf8_lossy(&output.stderr);
    let (elf_path, rendered_messages) = parse_cargo_messages(&stdout)?;

    let exit_code = output.status.code().unwrap_or(1);
    let diagnostics: String = if rendered_messages.is_empty() {
        stderr.into_owned()
    } else if stderr.is_empty() {
        rendered_messages.join("\n")
    } else {
        format!("{}\n{}", stderr, rendered_messages.join("\n"))
    };

    if exit_code != 0 && !resolved.allow_build_failure {
        anyhow::bail!("firmware build failed with exit code {exit_code}\n{diagnostics}");
    }

    Ok(FirmwareBuildCapture {
        exit_code,
        diagnostics,
        map_path: resolved.map_path.clone(),
        elf_path,
    })
}

fn cargo_build_args(resolved: &ResolvedReport, workspace_root: &Utf8Path) -> Vec<String> {
    let manifest_path = Utf8PathBuf::from(format!("{}/Cargo.toml", resolved.package));
    let mut cargo_args = vec!["build".to_owned(), "--locked".to_owned()];

    if workspace_root.join(&manifest_path).is_file() {
        cargo_args.push("--manifest-path".to_owned());
        cargo_args.push(manifest_path.to_string());
    } else {
        cargo_args.push("-p".to_owned());
        cargo_args.push(resolved.package.clone());
    }

    cargo_args.extend([
        "--target".to_owned(),
        resolved.target.clone(),
        "--message-format=json".to_owned(),
    ]);

    match resolved.profile.as_str() {
        "release" => cargo_args.push("--release".to_owned()),
        "dev" => {}
        other => {
            cargo_args.push("--profile".to_owned());
            cargo_args.push(other.to_owned());
        }
    }

    if resolved.no_default_features {
        cargo_args.push("--no-default-features".to_owned());
    }
    if !resolved.features.is_empty() {
        cargo_args.push("--features".to_owned());
        cargo_args.push(resolved.features.join(","));
    }

    cargo_args
}

fn parse_cargo_messages(stdout: &str) -> Result<(Option<Utf8PathBuf>, Vec<String>)> {
    let mut elf_path = None;
    let mut rendered_messages = Vec::new();

    for message in Message::parse_stream(stdout.as_bytes()) {
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
    FirmwareLinkerReport {
        ram_overflow_bytes: capture_bytes(&RAM_OVERFLOW_RE, output),
        uninit_overflow_bytes: capture_bytes(&UNINIT_OVERFLOW_RE, output),
        stack_placement_failed: STACK_PLACEMENT_RE.is_match(output),
    }
}

fn collect_sections(
    elf_path: Option<&Utf8Path>,
    map_path: &Utf8Path,
) -> Result<SectionReadout> {
    if let Some(elf_path) = elf_path
        && elf_path.is_file()
    {
        return read_elf_sections(elf_path);
    }

    if map_path.is_file() {
        return read_map_sections(map_path);
    }

    Ok(SectionReadout {
        sections: BTreeMap::new(),
        source: SectionSource::None,
    })
}

fn capture_bytes(regex: &Regex, text: &str) -> u64 {
    regex
        .captures(text)
        .and_then(|captures| captures.name("bytes"))
        .and_then(|capture| capture.as_str().parse().ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ResolvedReport;

    fn resolved_for(
        package: &str,
        target: &str,
        features: &[&str],
        no_default: bool,
    ) -> ResolvedReport {
        ResolvedReport {
            package: package.to_owned(),
            target: target.to_owned(),
            profile: "release".to_owned(),
            features: features.iter().map(|s| (*s).to_owned()).collect(),
            no_default_features: no_default,
            map_path: Utf8PathBuf::from("/tmp/firmware.map"),
            output: None,
            allow_build_failure: false,
            top_symbols: 20,
        }
    }

    #[test]
    fn cargo_build_args_emits_release_flag_and_features() {
        let resolved = resolved_for("demo", "thumbv7em-none-eabihf", &["a", "b"], true);
        let workspace = Utf8PathBuf::from("/nonexistent");
        let args = cargo_build_args(&resolved, &workspace);
        assert_eq!(
            args,
            vec![
                "build",
                "--locked",
                "-p",
                "demo",
                "--target",
                "thumbv7em-none-eabihf",
                "--message-format=json",
                "--release",
                "--no-default-features",
                "--features",
                "a,b",
            ]
        );
    }

    #[test]
    fn cargo_build_args_omits_features_when_empty() {
        let resolved = resolved_for("demo", "thumbv7em-none-eabihf", &[], false);
        let workspace = Utf8PathBuf::from("/nonexistent");
        let args = cargo_build_args(&resolved, &workspace);
        assert!(!args.contains(&"--features".to_owned()));
        assert!(!args.contains(&"--no-default-features".to_owned()));
    }
}

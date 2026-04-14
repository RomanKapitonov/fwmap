use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::Read,
    process::Command,
};

use ar::Archive;
use camino::{Utf8Path, Utf8PathBuf};
use cargo_metadata::{Message, TargetKind};
use object::{Object, ObjectSection, ObjectSymbol};

use crate::{
    budget::SizeProbeBudget,
    cli::{SizeProbeReportArgs, SizeProbeValidateArgs},
    report::SizeProbeReport,
};

const SIZE_PROBE_PACKAGE: &str = "size-probe";
const SYMBOL_PREFIX: &str = "__size_probe__";
const TYPE_SIZES_CATEGORY: &str = "type_sizes";
const RESOURCE_COUNTS_CATEGORY: &str = "resource_counts";
const RUNTIME_CAPS_CATEGORY: &str = "runtime_caps";
const META_CATEGORY: &str = "meta";
const PROBE_NAME: &str = "experimental_dag_demo";

pub fn collect_size_probe_report(args: &SizeProbeReportArgs) -> Result<SizeProbeReport, String> {
    let artifact = build_size_probe_rlib(&args.target)?;
    read_size_probe_report(&artifact, &args.target)
}

pub fn collect_size_probe_report_for_validation(
    args: &SizeProbeValidateArgs,
) -> Result<SizeProbeReport, String> {
    let artifact = build_size_probe_rlib(&args.target)?;
    read_size_probe_report(&artifact, &args.target)
}

pub fn validate_size_probe_budget(
    report: &SizeProbeReport,
    budget: &SizeProbeBudget,
) -> Vec<String> {
    let mut errors = Vec::new();

    if let Some(expected) = budget.pointer_width_bits
        && report.pointer_width_bits != expected
    {
        errors.push(format!(
            "pointer width mismatch: actual={} expected={expected}",
            report.pointer_width_bits
        ));
    }

    for (name, limit) in &budget.max_type_sizes {
        match report.type_sizes.get(name) {
            Some(actual) if actual > limit => errors.push(format!(
                "type size budget exceeded for [{name}]: actual={actual} limit={limit}"
            )),
            Some(_) => {}
            None => errors.push(format!("missing type size [{name}] in size-probe report")),
        }
    }

    for (name, expected) in &budget.exact_resource_counts {
        match report.resource_counts.get(name) {
            Some(actual) if actual != expected => errors.push(format!(
                "resource count mismatch for [{name}]: actual={actual} expected={expected}"
            )),
            Some(_) => {}
            None => errors.push(format!(
                "missing resource count [{name}] in size-probe report"
            )),
        }
    }

    for (name, expected) in &budget.exact_runtime_caps {
        match report.runtime_caps.get(name) {
            Some(actual) if actual != expected => errors.push(format!(
                "runtime cap mismatch for [{name}]: actual={actual} expected={expected}"
            )),
            Some(_) => {}
            None => errors.push(format!("missing runtime cap [{name}] in size-probe report")),
        }
    }

    errors
}

pub fn write_json_report(path: Option<&Utf8Path>, report: &SizeProbeReport) -> Result<(), String> {
    let rendered = serde_json::to_string_pretty(report)
        .map_err(|err| format!("failed to serialize size-probe report: {err}"))?;

    if let Some(path) = path {
        fs::write(path, rendered).map_err(|err| format!("failed to write {path}: {err}"))?;
    } else {
        println!("{rendered}");
    }

    Ok(())
}

fn build_size_probe_rlib(target: &str) -> Result<Utf8PathBuf, String> {
    let output = Command::new("cargo")
        .args([
            "build",
            "-q",
            "-p",
            SIZE_PROBE_PACKAGE,
            "--lib",
            "--target",
            target,
            "--message-format=json",
        ])
        .current_dir(repo_root())
        .output()
        .map_err(|err| format!("failed to build target size-probe: {err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "target size-probe build failed with exit code {}: {}",
            output.status.code().unwrap_or_default(),
            stderr.trim()
        ));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|err| format!("size-probe build output was not valid UTF-8: {err}"))?;
    parse_rlib_path(&stdout)
}

fn parse_rlib_path(stdout: &str) -> Result<Utf8PathBuf, String> {
    let cursor = std::io::Cursor::new(stdout.as_bytes());
    let mut rlib_path = None;

    for message in Message::parse_stream(cursor) {
        let message =
            message.map_err(|err| format!("failed to parse cargo JSON message: {err}"))?;
        if let Message::CompilerArtifact(artifact) = message
            && artifact
                .target
                .kind
                .iter()
                .any(|kind| kind == &TargetKind::Lib)
        {
            for filename in artifact.filenames {
                if filename.as_str().ends_with(".rlib") {
                    rlib_path = Some(filename);
                }
            }
        }
    }

    rlib_path.ok_or_else(|| "size-probe build did not produce an .rlib artifact".to_owned())
}

fn read_size_probe_report(rlib_path: &Utf8Path, target: &str) -> Result<SizeProbeReport, String> {
    let file = File::open(rlib_path).map_err(|err| format!("failed to open {rlib_path}: {err}"))?;
    let mut archive = Archive::new(file);
    let mut pointer_width_bits = None;
    let mut schema_version = None;
    let mut type_sizes = BTreeMap::new();
    let mut resource_counts = BTreeMap::new();
    let mut runtime_caps = BTreeMap::new();

    while let Some(entry_result) = archive.next_entry() {
        let mut entry =
            entry_result.map_err(|err| format!("failed to read archive entry: {err}"))?;
        let mut data = Vec::new();
        entry
            .read_to_end(&mut data)
            .map_err(|err| format!("failed to read archive payload: {err}"))?;

        let object = match object::File::parse(data.as_slice()) {
            Ok(object) => object,
            Err(_) => continue,
        };

        for symbol in object.symbols() {
            let Ok(name) = symbol.name() else {
                continue;
            };
            if !name.starts_with(SYMBOL_PREFIX) {
                continue;
            }
            if !symbol.is_definition() {
                continue;
            }

            let value = read_symbol_u64(&object, &symbol)?;
            let Some((category, metric_name)) = parse_symbol_name(name) else {
                continue;
            };

            match category {
                META_CATEGORY if metric_name == "schema_version" => {
                    schema_version = Some(value as u32)
                }
                META_CATEGORY if metric_name == "pointer_width_bits" => {
                    pointer_width_bits = Some(value as u32)
                }
                TYPE_SIZES_CATEGORY => {
                    type_sizes.insert(metric_name.to_owned(), value);
                }
                RESOURCE_COUNTS_CATEGORY => {
                    resource_counts.insert(metric_name.to_owned(), value);
                }
                RUNTIME_CAPS_CATEGORY => {
                    runtime_caps.insert(metric_name.to_owned(), value);
                }
                _ => {}
            }
        }
    }

    Ok(SizeProbeReport {
        schema_version: schema_version.unwrap_or(1),
        probe: PROBE_NAME.to_owned(),
        target: target.to_owned(),
        pointer_width_bits: pointer_width_bits
            .ok_or_else(|| "missing pointer_width_bits in target size-probe artifact".to_owned())?,
        type_sizes,
        resource_counts,
        runtime_caps,
    })
}

fn read_symbol_u64<'data, 'file>(
    object: &'file object::File<'data>,
    symbol: &object::Symbol<'data, 'file>,
) -> Result<u64, String> {
    let section_index = symbol.section_index().ok_or_else(|| {
        format!(
            "symbol `{}` is missing a section",
            symbol.name().unwrap_or("?")
        )
    })?;
    let section = object
        .section_by_index(section_index)
        .map_err(|err| format!("failed to read symbol section: {err}"))?;
    let data = section
        .data()
        .map_err(|err| format!("failed to read symbol section data: {err}"))?;
    let start = symbol.address().saturating_sub(section.address()) as usize;
    let end = start + 8;
    let bytes = data.get(start..end).ok_or_else(|| {
        format!(
            "symbol `{}` did not contain an inline u64 payload",
            symbol.name().unwrap_or("?")
        )
    })?;
    let bytes: [u8; 8] = bytes
        .try_into()
        .map_err(|_| "failed to decode u64 symbol payload".to_owned())?;
    Ok(u64::from_le_bytes(bytes))
}

fn parse_symbol_name(name: &str) -> Option<(&str, &str)> {
    let tail = name.strip_prefix(SYMBOL_PREFIX)?;
    let (category, metric_name) = tail.split_once("__")?;
    Some((category, metric_name))
}

fn repo_root() -> Utf8PathBuf {
    Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives under the workspace root")
        .to_owned()
}

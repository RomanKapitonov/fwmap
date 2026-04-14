use std::{collections::BTreeMap, fs};

use camino::Utf8Path;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct SizeProbeBudget {
    pub pointer_width_bits: Option<u32>,
    #[serde(default)]
    pub max_type_sizes: BTreeMap<String, u64>,
    #[serde(default)]
    pub exact_resource_counts: BTreeMap<String, u64>,
    #[serde(default)]
    pub exact_runtime_caps: BTreeMap<String, u64>,
}

pub fn load_size_probe_budget(path: &Utf8Path) -> Result<SizeProbeBudget, String> {
    let raw = fs::read_to_string(path).map_err(|err| format!("failed to read {path}: {err}"))?;
    serde_json::from_str(&raw).map_err(|err| format!("failed to parse {path}: {err}"))
}

#[derive(Debug, Clone, Deserialize)]
pub struct FirmwareMemoryBudget {
    pub require_successful_build: bool,
    pub require_valid_stack_placement: bool,
    pub max_ram_overflow_bytes: Option<u64>,
    pub max_uninit_overflow_bytes: Option<u64>,
    #[serde(default)]
    pub max_section_bytes: BTreeMap<String, u64>,
    #[serde(default)]
    pub max_symbol_prefix_bytes: BTreeMap<String, u64>,
}

pub fn load_firmware_memory_budget(path: &Utf8Path) -> Result<FirmwareMemoryBudget, String> {
    let raw = fs::read_to_string(path).map_err(|err| format!("failed to read {path}: {err}"))?;
    serde_json::from_str(&raw).map_err(|err| format!("failed to parse {path}: {err}"))
}

use std::collections::BTreeMap;
use std::fs;

use camino::Utf8Path;
use serde::Deserialize;

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

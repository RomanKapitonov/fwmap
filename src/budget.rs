use std::{collections::BTreeMap, fs};

use anyhow::{Context, Result};
use camino::Utf8Path;
use serde::Deserialize;

use crate::section::Section;

#[derive(Debug, Clone, Deserialize)]
pub struct FirmwareMemoryBudget {
    pub require_successful_build: bool,
    pub require_valid_stack_placement: bool,
    pub max_ram_overflow_bytes: Option<u64>,
    pub max_uninit_overflow_bytes: Option<u64>,
    #[serde(default)]
    pub max_section_bytes: BTreeMap<Section, u64>,
    #[serde(default)]
    pub max_symbol_prefix_bytes: BTreeMap<String, u64>,
}

pub fn load_firmware_memory_budget(path: &Utf8Path) -> Result<FirmwareMemoryBudget> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read {path}"))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse {path}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;

    #[test]
    fn missing_budget_file_error_chains_with_path_context() {
        let path = Utf8PathBuf::from("nonexistent-budget.json");
        let err = load_firmware_memory_budget(&path).unwrap_err();
        let chain: Vec<String> = err.chain().map(|e| e.to_string()).collect();
        assert!(
            chain.iter().any(|msg| msg.contains("nonexistent-budget.json")),
            "expected path in error chain, got: {chain:?}"
        );
    }
}

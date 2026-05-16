use std::{collections::BTreeMap, fs};

use anyhow::{Context, Result};
use camino::Utf8Path;
use serde::Deserialize;

use crate::section::Section;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "snake_case")]
pub struct FirmwareMemoryBudget {
    pub require_successful_build: bool,
    pub require_valid_stack_placement: bool,
    pub max_ram_overflow_bytes: u64,
    pub max_uninit_overflow_bytes: u64,
    pub max_section_bytes: BTreeMap<Section, u64>,
    pub max_symbol_prefix_bytes: BTreeMap<String, u64>,
    pub required_symbols: BTreeMap<String, u64>,
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
        let chain: Vec<String> = err.chain().map(ToString::to_string).collect();
        assert!(
            chain.iter().any(|msg| msg.contains("nonexistent-budget.json")),
            "expected path in error chain, got: {chain:?}"
        );
    }

    #[test]
    fn empty_budget_parses_with_defaults() {
        let budget: FirmwareMemoryBudget = serde_json::from_str("{}").unwrap();
        assert!(!budget.require_successful_build);
        assert!(!budget.require_valid_stack_placement);
        assert_eq!(budget.max_ram_overflow_bytes, 0);
        assert_eq!(budget.max_uninit_overflow_bytes, 0);
        assert!(budget.max_section_bytes.is_empty());
        assert!(budget.max_symbol_prefix_bytes.is_empty());
        assert!(budget.required_symbols.is_empty());
    }

    #[test]
    fn unknown_field_rejected() {
        let result: serde_json::Result<FirmwareMemoryBudget> =
            serde_json::from_str(r#"{"unknown_field": true}"#);
        assert!(result.is_err());
    }
}

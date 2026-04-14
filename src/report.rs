use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeProbeReport {
    pub schema_version: u32,
    pub probe: String,
    pub target: String,
    pub pointer_width_bits: u32,
    pub type_sizes: BTreeMap<String, u64>,
    pub resource_counts: BTreeMap<String, u64>,
    pub runtime_caps: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareMemoryReport {
    pub schema_version: u32,
    pub build: FirmwareBuildReport,
    pub linker: FirmwareLinkerReport,
    pub sections: BTreeMap<String, u64>,
    pub top_symbols: Vec<FirmwareSymbolReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareBuildReport {
    pub target: String,
    pub profile: String,
    pub features: Vec<String>,
    pub succeeded: bool,
    pub exit_code: i32,
    pub map_path: String,
    pub elf_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareLinkerReport {
    pub ram_overflow_bytes: u64,
    pub uninit_overflow_bytes: u64,
    pub stack_placement_failed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareSymbolReport {
    pub section: String,
    pub size_bytes: u64,
    pub address: String,
    pub symbol: String,
}

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::address::HexAddress;
use crate::section::Section;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareMemoryReport {
    pub build: FirmwareBuildReport,
    pub linker: FirmwareLinkerReport,
    pub sections: BTreeMap<Section, u64>,
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
    pub section_source: SectionSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SectionSource {
    Elf,
    Map,
    None,
}

#[derive(Debug)]
pub struct SectionReadout {
    pub sections: BTreeMap<Section, u64>,
    pub source: SectionSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareLinkerReport {
    pub ram_overflow_bytes: u64,
    pub uninit_overflow_bytes: u64,
    pub stack_placement_failed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareSymbolReport {
    pub section: Section,
    pub size_bytes: u64,
    pub address: HexAddress,
    pub symbol: String,
}

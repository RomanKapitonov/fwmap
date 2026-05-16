use std::str::FromStr;
use std::sync::LazyLock;
use std::{collections::BTreeMap, fs};

use anyhow::{Context, Result};
use camino::Utf8Path;
use regex::Regex;
use rustc_demangle::try_demangle;

use crate::address::HexAddress;
use crate::report::{FirmwareSymbolReport, SectionReadout, SectionSource};
use crate::section::Section;

static MAP_SECTION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(?P<vma>[0-9a-f]+)\s+(?P<lma>[0-9a-f]+)\s+(?P<size>[0-9a-f]+)\s+\d+\s+(?P<name>\.[^\s]+)$",
    )
    .expect("section regex is valid")
});
static MAP_ENTRY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(?P<vma>[0-9a-f]+)\s+(?P<lma>[0-9a-f]+)\s+(?P<size>[0-9a-f]+)\s+\d+\s+(?P<object>.+):\((?P<section>\.[^)]+)\)$",
    )
    .expect("entry regex is valid")
});
static MAP_SYMBOL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(?P<vma>[0-9a-f]+)\s+(?P<lma>[0-9a-f]+)\s+(?P<size>[0-9a-f]+)\s+\d+\s+(?P<symbol>.+)$",
    )
    .expect("symbol regex is valid")
});

const TRACKED_SYMBOL_SECTIONS: &[Section] = &[
    Section::Bss,
    Section::Data,
    Section::Ccmram,
    Section::Uninit,
    Section::Sdram,
];

pub fn read_map_sections(path: &Utf8Path) -> Result<SectionReadout> {
    let raw = fs::read_to_string(path).with_context(|| format!("failed to read {path}"))?;
    let mut sections = BTreeMap::new();

    for line in raw.lines() {
        let Some(captures) = MAP_SECTION_RE.captures(line) else {
            continue;
        };
        let Some(name) = captures.name("name").map(|capture| capture.as_str()) else {
            continue;
        };
        let Some(section) = Section::ALL
            .iter()
            .copied()
            .find(|candidate| name == candidate.as_linker_name())
        else {
            continue;
        };
        let size = parse_hex(
            captures
                .name("size")
                .map(|capture| capture.as_str())
                .unwrap_or_default(),
        )?;
        sections.insert(section, size);
    }

    Ok(SectionReadout {
        sections,
        source: SectionSource::Map,
    })
}

pub fn read_top_symbols(path: &Utf8Path, top: usize) -> Result<Vec<FirmwareSymbolReport>> {
    let raw = fs::read_to_string(path).with_context(|| format!("failed to read {path}"))?;
    let lines: Vec<&str> = raw.lines().collect();
    let mut symbols = Vec::new();

    for window in lines.windows(2) {
        let line = window[0];
        let next = window[1];
        let Some(entry) = MAP_ENTRY_RE.captures(line) else {
            continue;
        };

        let full_section = entry
            .name("section")
            .map(|capture| capture.as_str())
            .unwrap_or_default();
        let Some(section) = Section::ALL
            .iter()
            .copied()
            .find(|candidate| full_section.starts_with(candidate.as_linker_name()))
        else {
            continue;
        };
        if !TRACKED_SYMBOL_SECTIONS.contains(&section) {
            continue;
        }

        let address_hex = entry
            .name("vma")
            .map(|capture| capture.as_str())
            .unwrap_or_default();
        let address = HexAddress::from_str(address_hex)
            .with_context(|| format!("failed to parse address `{address_hex}`"))?;
        let size_hex = entry
            .name("size")
            .map(|capture| capture.as_str())
            .unwrap_or_default();
        let size_bytes = parse_hex(size_hex)?;
        let mut symbol = full_section.to_owned();

        if let Some(next_symbol) = MAP_SYMBOL_RE.captures(next) {
            let next_vma_matches = next_symbol
                .name("vma")
                .map(|capture| capture.as_str())
                .is_some_and(|vma_str| HexAddress::from_str(vma_str).ok() == Some(address));
            let next_size_matches = next_symbol
                .name("size")
                .map(|capture| capture.as_str())
                .is_some_and(|size_str| size_str == size_hex);
            if next_vma_matches && next_size_matches {
                symbol = next_symbol
                    .name("symbol")
                    .map(|capture| demangle_symbol(capture.as_str().trim()))
                    .unwrap_or(symbol);
            }
        }

        symbols.push(FirmwareSymbolReport {
            section,
            size_bytes,
            address,
            symbol,
        });
    }

    symbols.sort_by(|left, right| right.size_bytes.cmp(&left.size_bytes));
    symbols.truncate(top);
    Ok(symbols)
}

fn parse_hex(value: &str) -> Result<u64> {
    u64::from_str_radix(value, 16)
        .with_context(|| format!("failed to parse hex value `{value}`"))
}

fn demangle_symbol(symbol: &str) -> String {
    match try_demangle(symbol) {
        Ok(demangled) => demangled.to_string(),
        Err(_) => symbol.to_owned(),
    }
}

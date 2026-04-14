use std::{collections::BTreeMap, fs};

use camino::Utf8Path;
use regex::Regex;
use rustc_demangle::try_demangle;

use crate::report::FirmwareSymbolReport;

const TRACKED_SECTIONS: &[&str] = &[
    ".text", ".rodata", ".data", ".bss", ".uninit", ".ccmram", ".sdram",
];
const TRACKED_SYMBOL_ROOTS: &[&str] = &[".bss", ".data", ".ccmram", ".uninit", ".sdram"];

pub fn read_map_sections(path: &Utf8Path) -> Result<BTreeMap<String, u64>, String> {
    let raw = fs::read_to_string(path).map_err(|err| format!("failed to read {path}: {err}"))?;
    let regex = Regex::new(
        r"^(?P<vma>[0-9a-f]+)\s+(?P<lma>[0-9a-f]+)\s+(?P<size>[0-9a-f]+)\s+\d+\s+(?P<name>\.[^\s]+)$",
    )
    .expect("section regex is valid");
    let mut sections = BTreeMap::new();

    for line in raw.lines() {
        let Some(captures) = regex.captures(line) else {
            continue;
        };
        let Some(name) = captures.name("name").map(|capture| capture.as_str()) else {
            continue;
        };
        if !TRACKED_SECTIONS.contains(&name) {
            continue;
        }
        let size = parse_hex(
            captures
                .name("size")
                .map(|capture| capture.as_str())
                .unwrap_or_default(),
        )?;
        sections.insert(name.to_owned(), size);
    }

    Ok(sections)
}

pub fn read_top_symbols(path: &Utf8Path, top: usize) -> Result<Vec<FirmwareSymbolReport>, String> {
    let raw = fs::read_to_string(path).map_err(|err| format!("failed to read {path}: {err}"))?;
    let lines: Vec<&str> = raw.lines().collect();
    let entry_regex = Regex::new(
        r"^(?P<vma>[0-9a-f]+)\s+(?P<lma>[0-9a-f]+)\s+(?P<size>[0-9a-f]+)\s+\d+\s+(?P<object>.+):\((?P<section>\.[^)]+)\)$",
    )
    .expect("entry regex is valid");
    let symbol_regex = Regex::new(
        r"^(?P<vma>[0-9a-f]+)\s+(?P<lma>[0-9a-f]+)\s+(?P<size>[0-9a-f]+)\s+\d+\s+(?P<symbol>.+)$",
    )
    .expect("symbol regex is valid");
    let mut symbols = Vec::new();

    for window in lines.windows(2) {
        let line = window[0];
        let next = window[1];
        let Some(entry) = entry_regex.captures(line) else {
            continue;
        };

        let full_section = entry
            .name("section")
            .map(|capture| capture.as_str())
            .unwrap_or_default();
        let section_root = section_root(full_section);
        if !TRACKED_SYMBOL_ROOTS.contains(&section_root) {
            continue;
        }

        let address = entry
            .name("vma")
            .map(|capture| capture.as_str().to_owned())
            .unwrap_or_default();
        let size_hex = entry
            .name("size")
            .map(|capture| capture.as_str())
            .unwrap_or_default();
        let size_bytes = parse_hex(size_hex)?;
        let mut symbol = full_section.to_owned();

        if let Some(next_symbol) = symbol_regex.captures(next) {
            let next_vma = next_symbol
                .name("vma")
                .map(|capture| capture.as_str())
                .unwrap_or_default();
            let next_size = next_symbol
                .name("size")
                .map(|capture| capture.as_str())
                .unwrap_or_default();
            if next_vma == address && next_size == size_hex {
                symbol = next_symbol
                    .name("symbol")
                    .map(|capture| demangle_symbol(capture.as_str().trim()))
                    .unwrap_or(symbol);
            }
        }

        symbols.push(FirmwareSymbolReport {
            section: section_root.to_owned(),
            size_bytes,
            address,
            symbol,
        });
    }

    symbols.sort_by(|left, right| right.size_bytes.cmp(&left.size_bytes));
    symbols.truncate(top);
    Ok(symbols)
}

fn section_root(section: &str) -> &str {
    TRACKED_SECTIONS
        .iter()
        .copied()
        .find(|candidate| section.starts_with(candidate))
        .unwrap_or(section)
}

fn parse_hex(value: &str) -> Result<u64, String> {
    u64::from_str_radix(value, 16)
        .map_err(|err| format!("failed to parse hex value `{value}`: {err}"))
}

fn demangle_symbol(symbol: &str) -> String {
    match try_demangle(symbol) {
        Ok(demangled) => demangled.to_string(),
        Err(_) => symbol.to_owned(),
    }
}

use std::{collections::BTreeMap, fs};

use camino::Utf8Path;
use object::{Object, ObjectSection};

const TRACKED_SECTIONS: &[&str] = &[
    ".text", ".rodata", ".data", ".bss", ".uninit", ".ccmram", ".sdram",
];

pub fn read_elf_sections(path: &Utf8Path) -> Result<BTreeMap<String, u64>, String> {
    let bytes = fs::read(path).map_err(|err| format!("failed to read {path}: {err}"))?;
    let object = object::File::parse(bytes.as_slice())
        .map_err(|err| format!("failed to parse ELF {path}: {err}"))?;
    let mut sections = BTreeMap::new();

    for section in object.sections() {
        let Ok(name) = section.name() else {
            continue;
        };
        let Some(root) = TRACKED_SECTIONS
            .iter()
            .copied()
            .find(|candidate| name.starts_with(candidate))
        else {
            continue;
        };
        *sections.entry(root.to_owned()).or_insert(0) += section.size();
    }

    Ok(sections)
}

use std::{collections::BTreeMap, fs};

use anyhow::{Context, Result};
use camino::Utf8Path;
use object::{Object, ObjectSection};

use crate::report::{SectionReadout, SectionSource};
use crate::section::Section;

pub fn read_elf_sections(path: &Utf8Path) -> Result<SectionReadout> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {path}"))?;
    let object = object::File::parse(bytes.as_slice())
        .with_context(|| format!("failed to parse ELF {path}"))?;
    let mut sections = BTreeMap::new();

    for section in object.sections() {
        let Ok(name) = section.name() else {
            continue;
        };
        let Some(root) = Section::ALL
            .iter()
            .copied()
            .find(|candidate| name.starts_with(candidate.as_linker_name()))
        else {
            continue;
        };
        *sections.entry(root).or_insert(0) += section.size();
    }

    Ok(SectionReadout {
        sections,
        source: SectionSource::Elf,
    })
}

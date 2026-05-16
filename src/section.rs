use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum Section {
    Text,
    Rodata,
    Data,
    Bss,
    Uninit,
    Ccmram,
    Sdram,
}

impl Section {
    pub const ALL: &'static [Section] = &[
        Section::Text,
        Section::Rodata,
        Section::Data,
        Section::Bss,
        Section::Uninit,
        Section::Ccmram,
        Section::Sdram,
    ];

    pub fn as_linker_name(self) -> &'static str {
        match self {
            Self::Text => ".text",
            Self::Rodata => ".rodata",
            Self::Data => ".data",
            Self::Bss => ".bss",
            Self::Uninit => ".uninit",
            Self::Ccmram => ".ccmram",
            Self::Sdram => ".sdram",
        }
    }
}

impl FromStr for Section {
    type Err = UnknownSection;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            ".text" => Ok(Self::Text),
            ".rodata" => Ok(Self::Rodata),
            ".data" => Ok(Self::Data),
            ".bss" => Ok(Self::Bss),
            ".uninit" => Ok(Self::Uninit),
            ".ccmram" => Ok(Self::Ccmram),
            ".sdram" => Ok(Self::Sdram),
            other => Err(UnknownSection(other.to_owned())),
        }
    }
}

impl Display for Section {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_linker_name())
    }
}

#[derive(Debug, Clone)]
pub struct UnknownSection(pub String);

impl Display for UnknownSection {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "unknown section name: {}", self.0)
    }
}

impl std::error::Error for UnknownSection {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str_accepts_linker_names() {
        assert_eq!(Section::from_str(".bss").unwrap(), Section::Bss);
        assert_eq!(Section::from_str(".ccmram").unwrap(), Section::Ccmram);
        assert_eq!(Section::from_str(".text").unwrap(), Section::Text);
    }

    #[test]
    fn from_str_rejects_unknown_name() {
        let err = Section::from_str(".mystery").unwrap_err();
        assert!(err.to_string().contains(".mystery"));
    }

    #[test]
    fn display_writes_linker_name() {
        assert_eq!(Section::Bss.to_string(), ".bss");
        assert_eq!(Section::Sdram.to_string(), ".sdram");
    }

    #[test]
    fn all_lists_every_variant() {
        assert_eq!(Section::ALL.len(), 7);
        assert!(Section::ALL.contains(&Section::Text));
        assert!(Section::ALL.contains(&Section::Sdram));
    }

    #[test]
    fn serializes_as_variant_identifier() {
        let json = serde_json::to_string(&Section::Bss).unwrap();
        assert_eq!(json, "\"Bss\"");
    }

    #[test]
    fn deserializes_from_variant_identifier() {
        let section: Section = serde_json::from_str("\"Ccmram\"").unwrap();
        assert_eq!(section, Section::Ccmram);
    }
}

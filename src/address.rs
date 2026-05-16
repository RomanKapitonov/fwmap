use std::fmt::{self, Display, Formatter};
use std::num::ParseIntError;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HexAddress(pub u64);

impl FromStr for HexAddress {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.strip_prefix("0x").unwrap_or(s);
        u64::from_str_radix(trimmed, 16).map(HexAddress)
    }
}

impl Display for HexAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:08x}", self.0)
    }
}

impl Serialize for HexAddress {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_u64(self.0)
    }
}

impl<'de> Deserialize<'de> for HexAddress {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        u64::deserialize(de).map(HexAddress)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bare_hex() {
        assert_eq!(
            HexAddress::from_str("20000000").unwrap(),
            HexAddress(0x2000_0000)
        );
    }

    #[test]
    fn parses_with_0x_prefix() {
        assert_eq!(
            HexAddress::from_str("0x20000000").unwrap(),
            HexAddress(0x2000_0000)
        );
    }

    #[test]
    fn display_writes_0x_prefix_lowercase() {
        assert_eq!(HexAddress(0x2000_0000).to_string(), "0x20000000");
    }

    #[test]
    fn serializes_as_numeric_u64() {
        let json = serde_json::to_string(&HexAddress(0x2000_0000)).unwrap();
        assert_eq!(json, "536870912");
    }

    #[test]
    fn deserializes_from_numeric_u64() {
        let addr: HexAddress = serde_json::from_str("536870912").unwrap();
        assert_eq!(addr, HexAddress(0x2000_0000));
    }
}

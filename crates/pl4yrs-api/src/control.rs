use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Volume(u8);

impl Volume {
    pub const MAX: u8 = 100;

    pub fn new(value: u8) -> Result<Self, InvalidVolume> {
        if value > Self::MAX {
            return Err(InvalidVolume(value));
        }
        Ok(Self(value))
    }

    pub fn get(self) -> u8 {
        self.0
    }

    pub fn from_clamped(value: u8) -> Self {
        Self(value.min(Self::MAX))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidVolume(pub u8);

impl fmt::Display for InvalidVolume {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "volume {} is out of range (0-{})", self.0, Volume::MAX)
    }
}

impl std::error::Error for InvalidVolume {}

impl Serialize for Volume {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u8(self.0)
    }
}

impl<'a> Deserialize<'a> for Volume {
    fn deserialize<D: Deserializer<'a>>(d: D) -> Result<Self, D::Error> {
        let value = u8::deserialize(d)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetVolume {
    pub volume: Volume,
}

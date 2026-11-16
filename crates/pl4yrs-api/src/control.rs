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

impl<'de> Deserialize<'de> for Volume {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let value = u8::deserialize(d)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetVolume {
    pub volume: Volume,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn new_accepts_valid_values() {
        for value in [0u8, 42, 100] {
            assert_eq!(Volume::new(value).unwrap().get(), value);
        }
    }

    #[test]
    fn serializes_as_bare_number() {
        for value in [0u8, 42, 100] {
            let volume = Volume::new(value).unwrap();
            assert_eq!(serde_json::to_value(volume).unwrap(), json!(value));
        }
    }

    #[test]
    fn rejects_max_plus_one() {
        assert_eq!(Volume::new(101), Err(InvalidVolume(101)));
    }

    #[test]
    fn rejects_u8_max() {
        assert_eq!(Volume::new(255), Err(InvalidVolume(255)));
    }

    #[test]
    fn from_clamped_values() {
        for (input, expected) in [(0u8, 0u8), (42, 42), (100, 100), (101, 100), (255, 100)] {
            assert_eq!(Volume::from_clamped(input).get(), expected);
        }
    }

    #[test]
    fn deserialize_accepts_valid_values() {
        for value in [0u8, 42, 100] {
            let volume: Volume = serde_json::from_value(json!(value)).unwrap();
            assert_eq!(volume.get(), value);
        }
    }

    #[test]
    fn deserialize_rejects_out_of_range() {
        for value in [101u8, 255] {
            assert!(serde_json::from_value::<Volume>(json!(value)).is_err());
        }
    }

    #[test]
    fn deserialize_error_carries_range_message() {
        let err = serde_json::from_value::<Volume>(json!(101)).unwrap_err();
        assert!(err.to_string().contains("out of range"));
    }

    #[test]
    fn deserialize_rejects_wrong_types() {
        assert!(serde_json::from_value::<Volume>(json!(-1)).is_err());
        assert!(serde_json::from_value::<Volume>(json!("50")).is_err());
        assert!(serde_json::from_value::<Volume>(json!(50.5)).is_err());
    }

    #[test]
    fn serialize_deserialize_roundtrip() {
        let volume = Volume::new(42).unwrap();
        let json = serde_json::to_value(volume).unwrap();
        assert_eq!(serde_json::from_value::<Volume>(json).unwrap(), volume);
    }

    #[test]
    fn invalid_volume_display_includes_value_and_range() {
        assert_eq!(
            InvalidVolume(200).to_string(),
            "volume 200 is out of range (0-100)"
        );
    }

    #[test]
    fn set_volume_deserializes_request_shape() {
        let req: SetVolume = serde_json::from_value(json!({ "volume": 42 })).unwrap();
        assert_eq!(req.volume.get(), 42);
    }

    #[test]
    fn set_volume_serializes_request_shape() {
        let req = SetVolume {
            volume: Volume::new(42).unwrap(),
        };
        assert_eq!(serde_json::to_value(req).unwrap(), json!({ "volume": 42 }));
    }

    #[test]
    fn set_volume_deserialize_rejects_out_of_range() {
        let err = serde_json::from_value::<SetVolume>(json!({ "volume": 200 })).unwrap_err();
        assert!(err.to_string().contains("out of range"));
    }

    #[test]
    fn set_volume_deserialize_rejects_missing_field() {
        assert!(serde_json::from_value::<SetVolume>(json!({})).is_err());
    }

    #[test]
    fn set_volume_deserialize_rejects_wrong_field_type() {
        assert!(serde_json::from_value::<SetVolume>(json!({ "volume": "42" })).is_err());
    }
}

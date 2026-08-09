use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: &str = "SEM33_R1_U16_KEY_VALUE_RECORDS_1";

pub mod u16_key_map {
    use std::collections::BTreeMap;

    use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize)]
    struct EntryRef<'a, V> {
        key: u16,
        value: &'a V,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Entry<V> {
        key: u16,
        value: V,
    }

    pub fn serialize<S, V>(map: &BTreeMap<u16, V>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        V: Serialize,
    {
        map.iter()
            .map(|(key, value)| EntryRef { key: *key, value })
            .collect::<Vec<_>>()
            .serialize(serializer)
    }

    pub fn deserialize<'de, D, V>(deserializer: D) -> Result<BTreeMap<u16, V>, D::Error>
    where
        D: Deserializer<'de>,
        V: Deserialize<'de>,
    {
        let entries = Vec::<Entry<V>>::deserialize(deserializer)?;
        let mut map = BTreeMap::new();
        for entry in entries {
            if map.insert(entry.key, entry.value).is_some() {
                return Err(D::Error::custom(format!(
                    "DUPLICATE_U16_TRANSPORT_KEY:{}",
                    entry.key
                )));
            }
        }
        Ok(map)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(serialize = "V: Serialize", deserialize = "V: Deserialize<'de>"))]
pub struct CanonicalU16Map<V>(#[serde(with = "u16_key_map")] pub BTreeMap<u16, V>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NestedCanary {
    pub label: String,
    pub empty: CanonicalU16Map<String>,
    pub maps: Vec<CanonicalU16Map<String>>,
    pub adjacent: bool,
}

pub fn valid_roundtrip_canary() -> Result<bool, String> {
    let keys = [0_u16, 1, 100, 255, 256, 32_767, 65_535];
    let map = CanonicalU16Map(
        keys.into_iter()
            .map(|key| (key, format!("VALUE_{key}")))
            .collect(),
    );
    let bytes = serde_json::to_vec(&map).map_err(|error| error.to_string())?;
    let decoded: CanonicalU16Map<String> =
        serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    Ok(decoded == map)
}

pub fn nested_roundtrip_canary() -> Result<bool, String> {
    let first = CanonicalU16Map(BTreeMap::from([(1, "ONE".into()), (100, "HUNDRED".into())]));
    let second = CanonicalU16Map(BTreeMap::from([(65_535, "MAX".into())]));
    let canary = NestedCanary {
        label: "UNRELATED_ADJACENT_FIELD".into(),
        empty: CanonicalU16Map(BTreeMap::new()),
        maps: vec![first, second],
        adjacent: true,
    };
    let bytes = serde_json::to_vec(&canary).map_err(|error| error.to_string())?;
    let decoded: NestedCanary =
        serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    Ok(decoded == canary)
}

pub fn invalid_payloads() -> Vec<(&'static str, &'static str)> {
    vec![
        ("negative", r#"[{"key":-1,"value":"X"}]"#),
        ("overflow", r#"[{"key":65536,"value":"X"}]"#),
        ("non_numeric", r#"[{"key":"abc","value":"X"}]"#),
        ("empty", r#"[{"key":"","value":"X"}]"#),
        ("fractional", r#"[{"key":1.5,"value":"X"}]"#),
        ("malformed", r#"[{"key":"1x","value":"X"}]"#),
        ("ambiguous_text", r#"[{"key":"01","value":"X"}]"#),
        (
            "duplicate",
            r#"[{"key":100,"value":"A"},{"key":100,"value":"B"}]"#,
        ),
    ]
}

pub fn invalid_rejection_canary() -> Vec<(String, bool, String)> {
    invalid_payloads()
        .into_iter()
        .map(
            |(name, payload)| match serde_json::from_str::<CanonicalU16Map<String>>(payload) {
                Ok(_) => (name.into(), false, "FAIL_OPEN".into()),
                Err(error) => (name.into(), true, error.to_string()),
            },
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_u16_keys_roundtrip_deterministically() {
        assert_eq!(valid_roundtrip_canary(), Ok(true));
    }

    #[test]
    fn invalid_u16_keys_fail_closed() {
        assert!(invalid_rejection_canary()
            .iter()
            .all(|(_, rejected, _)| *rejected));
    }

    #[test]
    fn nested_array_empty_multi_and_adjacent_cases_roundtrip() {
        assert_eq!(nested_roundtrip_canary(), Ok(true));
    }
}

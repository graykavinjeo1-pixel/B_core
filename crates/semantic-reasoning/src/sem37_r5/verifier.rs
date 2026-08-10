use serde_json::Value;
use sha2::{Digest, Sha256};

pub fn canonical_sha256(value: &Value) -> Result<String, String> {
    let bytes = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    let digest = Sha256::digest(bytes);
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

pub fn acceptance_diff(primary: &Value, secondary: &Value) -> u64 {
    let fields = [
        "status",
        "disposition",
        "level_a_pass",
        "level_b_pass",
        "level_c_pass",
        "level_d_pass",
        "level_e_pass",
        "level_f_pass",
        "level_g_pass",
        "level_h_pass",
    ];
    fields
        .iter()
        .filter(|field| primary.get(**field) != secondary.get(**field))
        .count() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn canonical_hash_is_stable() {
        let value = json!({"a": 1, "b": [2, 3]});
        assert_eq!(canonical_sha256(&value), canonical_sha256(&value));
    }
}

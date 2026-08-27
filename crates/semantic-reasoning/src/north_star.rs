//! Compile-time integrity contract for the immutable B_Core product purpose.

use crate::self_repair_contract::sha256;

pub const B_CORE_NORTH_STAR_SHA256: &str =
    "e20c1ef511d0f82df2fcb31ffa0b77b477c6cd9c50bcb8faf1cacc181944040f";

pub const B_CORE_NORTH_STAR_DOCUMENT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/B_CORE_NORTH_STAR.md"
));

pub fn north_star_sha256() -> String {
    sha256(B_CORE_NORTH_STAR_DOCUMENT.as_bytes())
}

pub fn north_star_integrity_locked() -> bool {
    north_star_sha256() == B_CORE_NORTH_STAR_SHA256
}

#[cfg(test)]
mod tests {
    use super::{north_star_integrity_locked, north_star_sha256, B_CORE_NORTH_STAR_SHA256};

    #[test]
    fn north_star_contract_is_content_locked() {
        assert_eq!(north_star_sha256(), B_CORE_NORTH_STAR_SHA256);
        assert!(north_star_integrity_locked());
    }
}

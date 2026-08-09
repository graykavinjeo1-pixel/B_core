use std::collections::BTreeMap;

use serde::{de, Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NumericAuthorityClass {
    ExactInteger,
    ExactDerivedRational,
    ExactEnumOrDiscrete,
    MeasuredFloat,
    DisplayOnlyFloat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ExactRational {
    numerator: u64,
    denominator: u64,
}

impl<'de> Deserialize<'de> for ExactRational {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct WireRational {
            numerator: u64,
            denominator: u64,
        }

        let wire = WireRational::deserialize(deserializer)?;
        Self::new(wire.numerator, wire.denominator).map_err(de::Error::custom)
    }
}

impl ExactRational {
    pub fn new(numerator: u64, denominator: u64) -> Result<Self, String> {
        if denominator == 0 {
            return Err("EXACT_RATIONAL_ZERO_DENOMINATOR".to_string());
        }
        let divisor = gcd(numerator, denominator);
        Ok(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    pub fn numerator(self) -> u64 {
        self.numerator
    }

    pub fn denominator(self) -> u64 {
        self.denominator
    }

    pub fn to_display_f64(self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }

    pub fn checked_product(self, factor: u64) -> Result<(u64, u64), String> {
        Ok((
            self.numerator
                .checked_mul(factor)
                .ok_or("EXACT_RATIONAL_NUMERATOR_OVERFLOW")?,
            self.denominator
                .checked_mul(factor)
                .ok_or("EXACT_RATIONAL_DENOMINATOR_OVERFLOW")?,
        ))
    }
}

fn gcd(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CanonicalFiniteF64 {
    ieee754_bits: u64,
}

impl<'de> Deserialize<'de> for CanonicalFiniteF64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct WireFloat {
            ieee754_bits: u64,
        }

        let wire = WireFloat::deserialize(deserializer)?;
        Self::from_bits(wire.ieee754_bits).map_err(de::Error::custom)
    }
}

impl CanonicalFiniteF64 {
    pub fn new(value: f64) -> Result<Self, String> {
        if !value.is_finite() {
            return Err("NONFINITE_FLOAT_FORBIDDEN".to_string());
        }
        Ok(Self {
            ieee754_bits: value.to_bits(),
        })
    }

    pub fn from_bits(bits: u64) -> Result<Self, String> {
        Self::new(f64::from_bits(bits))
    }

    pub fn bits(self) -> u64 {
        self.ieee754_bits
    }

    pub fn value(self) -> f64 {
        f64::from_bits(self.ieee754_bits)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "authority_class",
    content = "value",
    rename_all = "SCREAMING_SNAKE_CASE"
)]
pub enum CanonicalNumericValue {
    ExactInteger(u64),
    ExactDerivedRational(ExactRational),
    ExactEnumOrDiscrete(String),
    MeasuredFloat(CanonicalFiniteF64),
    DisplayOnlyFloat(CanonicalFiniteF64),
}

impl CanonicalNumericValue {
    pub fn authority_class(&self) -> NumericAuthorityClass {
        match self {
            Self::ExactInteger(_) => NumericAuthorityClass::ExactInteger,
            Self::ExactDerivedRational(_) => NumericAuthorityClass::ExactDerivedRational,
            Self::ExactEnumOrDiscrete(_) => NumericAuthorityClass::ExactEnumOrDiscrete,
            Self::MeasuredFloat(_) => NumericAuthorityClass::MeasuredFloat,
            Self::DisplayOnlyFloat(_) => NumericAuthorityClass::DisplayOnlyFloat,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumericTransportMatrix {
    pub integer: CanonicalNumericValue,
    pub rational: CanonicalNumericValue,
    pub nested_ratios: Vec<Vec<ExactRational>>,
    pub ratio_array: Vec<ExactRational>,
    pub numeric_map: BTreeMap<String, CanonicalNumericValue>,
    pub optional_numeric: Option<CanonicalNumericValue>,
    pub empty_array: Vec<CanonicalNumericValue>,
    pub empty_map: BTreeMap<String, CanonicalNumericValue>,
    pub measured_float_canaries: Vec<CanonicalFiniteF64>,
    pub display_float: CanonicalNumericValue,
}

pub fn canonical_transport_matrix() -> Result<NumericTransportMatrix, String> {
    let float_values = [
        0.0,
        -0.0,
        f64::MIN_POSITIVE,
        f64::MAX,
        f64::from_bits(1),
        1.0,
        f64::from_bits(1.0_f64.to_bits() + 1),
        58.0 / 15.0,
        0.1,
    ];
    let measured_float_canaries = float_values
        .into_iter()
        .map(CanonicalFiniteF64::new)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(NumericTransportMatrix {
        integer: CanonicalNumericValue::ExactInteger(65_536),
        rational: CanonicalNumericValue::ExactDerivedRational(ExactRational::new(116, 30)?),
        nested_ratios: vec![vec![
            ExactRational::new(58, 15)?,
            ExactRational::new(44, 2)?,
        ]],
        ratio_array: vec![ExactRational::new(50, 3)?, ExactRational::new(67, 5)?],
        numeric_map: [
            (
                "work".to_string(),
                CanonicalNumericValue::ExactInteger(11_753),
            ),
            (
                "ratio".to_string(),
                CanonicalNumericValue::ExactDerivedRational(ExactRational::new(58, 15)?),
            ),
            (
                "measurement".to_string(),
                CanonicalNumericValue::MeasuredFloat(CanonicalFiniteF64::new(0.1)?),
            ),
        ]
        .into_iter()
        .collect(),
        optional_numeric: Some(CanonicalNumericValue::ExactInteger(0)),
        empty_array: Vec::new(),
        empty_map: BTreeMap::new(),
        measured_float_canaries,
        display_float: CanonicalNumericValue::DisplayOnlyFloat(CanonicalFiniteF64::new(
            58.0 / 15.0,
        )?),
    })
}

pub fn validate_matrix(matrix: &NumericTransportMatrix) -> Result<(), String> {
    if matrix.rational.authority_class() != NumericAuthorityClass::ExactDerivedRational
        || matrix.display_float.authority_class() != NumericAuthorityClass::DisplayOnlyFloat
        || !matrix.empty_array.is_empty()
        || !matrix.empty_map.is_empty()
        || matrix.measured_float_canaries.is_empty()
    {
        return Err("NUMERIC_MATRIX_AUTHORITY_CLASS_MISMATCH".to_string());
    }
    for canary in &matrix.measured_float_canaries {
        CanonicalFiniteF64::from_bits(canary.bits())?;
    }
    for ratio in matrix
        .nested_ratios
        .iter()
        .flatten()
        .chain(matrix.ratio_array.iter())
    {
        if *ratio != ExactRational::new(ratio.numerator(), ratio.denominator())? {
            return Err("NON_CANONICAL_EXACT_RATIONAL".to_string());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equivalent_ratios_have_identical_authority() {
        assert_eq!(
            ExactRational::new(116, 30).unwrap(),
            ExactRational::new(58, 15).unwrap()
        );
    }

    #[test]
    fn rational_roundtrip_is_exact_and_reduced() {
        let rational = ExactRational::new(116, 30).unwrap();
        let encoded = serde_json::to_vec(&rational).unwrap();
        let decoded: ExactRational = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, ExactRational::new(58, 15).unwrap());
        assert_eq!(decoded.numerator(), 58);
        assert_eq!(decoded.denominator(), 15);
    }

    #[test]
    fn zero_denominator_and_overflow_fail_closed() {
        assert!(ExactRational::new(1, 0).is_err());
        assert!(ExactRational::new(u64::MAX, 1)
            .unwrap()
            .checked_product(2)
            .is_err());
    }

    #[test]
    fn finite_ieee_canaries_roundtrip_by_bits() {
        for canary in canonical_transport_matrix()
            .unwrap()
            .measured_float_canaries
        {
            let encoded = serde_json::to_vec(&canary).unwrap();
            let decoded: CanonicalFiniteF64 = serde_json::from_slice(&encoded).unwrap();
            assert_eq!(decoded.bits(), canary.bits());
        }
    }

    #[test]
    fn nonfinite_values_fail_closed() {
        assert!(CanonicalFiniteF64::new(f64::NAN).is_err());
        assert!(CanonicalFiniteF64::new(f64::INFINITY).is_err());
        assert!(CanonicalFiniteF64::new(f64::NEG_INFINITY).is_err());
        let nan_payload = format!(r#"{{"ieee754_bits":{}}}"#, f64::NAN.to_bits());
        assert!(serde_json::from_str::<CanonicalFiniteF64>(&nan_payload).is_err());
    }

    #[test]
    fn malformed_wire_rational_fails_closed_and_equivalent_wire_is_reduced() {
        assert!(
            serde_json::from_str::<ExactRational>(r#"{"numerator":1,"denominator":0}"#).is_err()
        );
        let reduced: ExactRational =
            serde_json::from_str(r#"{"numerator":116,"denominator":30}"#).unwrap();
        assert_eq!(reduced, ExactRational::new(58, 15).unwrap());
    }

    #[test]
    fn mixed_numeric_matrix_roundtrip_is_exact() {
        let matrix = canonical_transport_matrix().unwrap();
        let encoded = serde_json::to_vec(&matrix).unwrap();
        let decoded: NumericTransportMatrix = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, matrix);
        validate_matrix(&decoded).unwrap();
    }
}

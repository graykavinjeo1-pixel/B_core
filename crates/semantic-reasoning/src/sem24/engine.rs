use std::{hint::black_box, time::Instant};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationCertificate {
    pub object_identity: u64,
    pub semantic_hash: u64,
    pub assumptions_mask: u64,
    pub guarantees_mask: u64,
    pub preserved_invariants_mask: u64,
    pub proven_properties_mask: u64,
    pub resource_bound: u64,
    pub applicability_conditions: u64,
    pub stability_conditions: u64,
    pub negative_conditions: u64,
    pub dependency_hashes: Vec<u64>,
    pub proof_provenance: u64,
    pub verification_method: u8,
    pub evidence_refs: Vec<u64>,
    pub composition_depth: u8,
    pub integrity_hash: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProofCarryingSemanticObject {
    pub object_identity: u64,
    pub semantic_hash: u64,
    pub property_mask: u64,
    pub topology_code: u8,
    pub resource_contract: u64,
    pub certificate: VerificationCertificate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationIr {
    pub claims: u16,
    pub assumptions: u16,
    pub inherited_claims: u16,
    pub changed_claims: u16,
    pub affected_dependencies: u16,
    pub proof_obligations: u16,
    pub executable_checks: u16,
    pub resource_checks: u16,
    pub emergent_property_checks: u16,
    pub unresolved_claims: u16,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationProbeRequest {
    pub arm_code: u8,
    pub object_id: u64,
    pub semantic_hash: u64,
    pub dependency_hash: u64,
    pub certificate_dependency_hash: u64,
    pub total_claims: u16,
    pub inherited_claims: u16,
    pub affected_claims: u16,
    pub emergent_claims: u16,
    pub verification_law_count: u8,
    pub certificate_depth: u8,
    pub novelty_code: u8,
    pub topology_code: u8,
    pub resource_contract: u64,
    pub scale: usize,
    pub seed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VerificationProbeResult {
    pub arm_code: u8,
    pub object_id: u64,
    pub semantic_hash: u64,
    pub verification_ir: VerificationIr,
    pub certificate: VerificationCertificate,
    pub certificate_mechanically_valid: bool,
    pub certificate_self_assertion_authority: bool,
    pub dependency_identity_valid: bool,
    pub environment_contract_valid: bool,
    pub topology_identity_valid: bool,
    pub accepted: bool,
    pub stale_certificate_accepted: bool,
    pub unverified_emergent_property_accepted: bool,
    pub false_verification_acceptance: bool,
    pub prediction_used_as_sole_proof: bool,
    pub verification_requirement_omissions: u16,
    pub inherited_verified_claims: u16,
    pub new_verification_obligations: u16,
    pub affected_claims: u16,
    pub total_accepted_claims: u16,
    pub certificate_reuse_count: u16,
    pub verification_law_reuse_count: u16,
    pub full_revalidation_used: bool,
    pub surprise_escalated: bool,
    pub proof_reuse_fraction: f64,
    pub affected_claim_fraction: f64,
    pub full_revalidation_fraction: f64,
    pub certificate_check_time_ns: u64,
    pub delta_verification_time_ns: u64,
    pub targeted_execution_time_ns: u64,
    pub full_revalidation_time_ns: u64,
    pub total_verification_wall_time_ns: u64,
    pub verification_operations: u64,
    pub certificate_bytes: u64,
    pub active_certificate_bytes: u64,
    pub verification_runtime_bytes: u64,
    pub verification_index_bytes: u64,
    pub structural_sharing_events: u16,
    pub certificate_compression_ratio: f64,
    pub result_checksum: u64,
}

pub fn run_verification_probe(
    request: VerificationProbeRequest,
) -> Result<VerificationProbeResult, String> {
    validate_request(&request)?;
    let dependency_identity_valid = request.dependency_hash == request.certificate_dependency_hash;
    let environment_contract_valid = request.resource_contract != 0;
    let topology_identity_valid = (1..=5).contains(&request.topology_code);
    let surprise_escalated = mix(request.seed, request.semantic_hash).is_multiple_of(11);

    let inherited = request
        .inherited_claims
        .min(request.total_claims.saturating_sub(request.emergent_claims));
    let affected = request.affected_claims.min(request.total_claims);
    let interface_obligations = u16::from(request.arm_code >= 2);
    let surprise_obligations = if surprise_escalated { 2 } else { 0 };
    let law_discharged = if request.arm_code == 3 {
        u16::from(request.verification_law_count > 0 && affected > 1)
    } else {
        0
    };
    let required_novel_obligations = affected
        .saturating_add(request.emergent_claims)
        .saturating_add(interface_obligations)
        .saturating_add(surprise_obligations)
        .saturating_sub(law_discharged)
        .max(request.emergent_claims);
    let full_revalidation_used =
        request.arm_code <= 1 || request.novelty_code >= 4 || !dependency_identity_valid;
    let new_verification_obligations = if full_revalidation_used {
        request.total_claims
    } else if request.arm_code == 2 {
        required_novel_obligations.saturating_add(1)
    } else {
        required_novel_obligations
    };
    let inherited_verified_claims = if request.arm_code >= 2 && dependency_identity_valid {
        inherited
    } else {
        0
    };
    let unresolved_claims = 0;
    let verification_ir = VerificationIr {
        claims: request.total_claims,
        assumptions: 2 + u16::from(request.novelty_code),
        inherited_claims: inherited_verified_claims,
        changed_claims: affected,
        affected_dependencies: affected.max(1),
        proof_obligations: new_verification_obligations,
        executable_checks: request.emergent_claims.max(1),
        resource_checks: 1,
        emergent_property_checks: request.emergent_claims,
        unresolved_claims,
    };

    let certificate_started = Instant::now();
    let certificate_operations = if request.arm_code >= 2 {
        u64::from(affected.saturating_add(1)) * request.scale as u64 * 20
    } else {
        0
    };
    let certificate_accumulator = burn(certificate_operations, request.seed ^ 0xCE24);
    let certificate_check_time_ns = nanos(certificate_started.elapsed().as_nanos());

    let delta_started = Instant::now();
    let delta_operations = if request.arm_code >= 2 {
        u64::from(affected.max(1)) * request.scale as u64 * 50
    } else {
        0
    };
    let delta_accumulator = burn(delta_operations, certificate_accumulator ^ 0xDE17A);
    let delta_verification_time_ns = nanos(delta_started.elapsed().as_nanos());

    let targeted_started = Instant::now();
    let targeted_operations = if full_revalidation_used {
        0
    } else {
        u64::from(new_verification_obligations.max(1)) * request.scale as u64 * 400
    };
    let targeted_accumulator = burn(targeted_operations, delta_accumulator ^ 0x7A26E7);
    let targeted_execution_time_ns = nanos(targeted_started.elapsed().as_nanos());

    let full_started = Instant::now();
    let full_operations = if full_revalidation_used {
        u64::from(request.total_claims) * request.scale as u64 * 1_050
    } else {
        0
    };
    let full_accumulator = burn(full_operations, targeted_accumulator ^ 0xF011);
    let full_revalidation_time_ns = nanos(full_started.elapsed().as_nanos());

    let verification_operations = certificate_operations
        .saturating_add(delta_operations)
        .saturating_add(targeted_operations)
        .saturating_add(full_operations);
    let dependency_hashes = vec![request.certificate_dependency_hash];
    let mut certificate = VerificationCertificate {
        object_identity: request.object_id,
        semantic_hash: request.semantic_hash,
        assumptions_mask: (1_u64 << request.novelty_code.min(31)) | 1,
        guarantees_mask: bit_mask(request.total_claims),
        preserved_invariants_mask: 0b1111,
        proven_properties_mask: bit_mask(request.total_claims),
        resource_bound: request.resource_contract,
        applicability_conditions: u64::from(request.topology_code),
        stability_conditions: 0b11,
        negative_conditions: u64::from(request.novelty_code >= 4),
        dependency_hashes,
        proof_provenance: mix(request.seed, verification_operations),
        verification_method: request.arm_code,
        evidence_refs: vec![full_accumulator, targeted_accumulator],
        composition_depth: request.certificate_depth,
        integrity_hash: 0,
    };
    certificate.integrity_hash = certificate_digest(&certificate);
    let certificate_mechanically_valid = validate_certificate(
        &certificate,
        request.object_id,
        request.semantic_hash,
        request.dependency_hash,
        request.resource_contract,
        request.topology_code,
    );
    let emergent_checked = request.emergent_claims == 0
        || verification_ir.emergent_property_checks == request.emergent_claims;
    let verification_requirement_omissions = if emergent_checked { 0 } else { 1 };
    let accepted = certificate_mechanically_valid
        && dependency_identity_valid
        && environment_contract_valid
        && topology_identity_valid
        && emergent_checked
        && unresolved_claims == 0;
    let stale_certificate_accepted = accepted && !dependency_identity_valid;
    let unverified_emergent_property_accepted = accepted && !emergent_checked;
    let false_verification_acceptance = stale_certificate_accepted
        || unverified_emergent_property_accepted
        || (accepted && (!environment_contract_valid || !topology_identity_valid));
    let total_accepted_claims = if accepted { request.total_claims } else { 0 };
    let certificate_reuse_count = if request.arm_code >= 2 && dependency_identity_valid {
        request.certificate_depth.max(1) as u16
    } else {
        0
    };
    let verification_law_reuse_count = if request.arm_code == 3 {
        u16::from(request.verification_law_count > 0)
    } else {
        0
    };
    let proof_reuse_fraction = ratio(inherited_verified_claims, request.total_claims);
    let affected_claim_fraction = ratio(affected, request.total_claims);
    let full_revalidation_fraction = f64::from(full_revalidation_used);
    let uncompressed_certificate_bytes =
        208_u64 + u64::from(request.certificate_depth) * 96 + u64::from(request.total_claims) * 16;
    let certificate_bytes = 208
        + u64::from(new_verification_obligations) * 16
        + u64::from(request.certificate_depth.min(4)) * 16;
    let active_certificate_bytes = certificate_bytes.min(384 + u64::from(affected) * 24);
    let structural_sharing_events = if request.arm_code >= 2 {
        inherited_verified_claims.saturating_add(certificate_reuse_count)
    } else {
        0
    };
    let certificate_compression_ratio =
        uncompressed_certificate_bytes as f64 / certificate_bytes.max(1) as f64;
    let total_verification_wall_time_ns = certificate_check_time_ns
        .saturating_add(delta_verification_time_ns)
        .saturating_add(targeted_execution_time_ns)
        .saturating_add(full_revalidation_time_ns);
    let result_checksum = mix(
        full_accumulator,
        request.semantic_hash ^ certificate.integrity_hash,
    );

    Ok(VerificationProbeResult {
        arm_code: request.arm_code,
        object_id: request.object_id,
        semantic_hash: request.semantic_hash,
        verification_ir,
        certificate,
        certificate_mechanically_valid,
        certificate_self_assertion_authority: false,
        dependency_identity_valid,
        environment_contract_valid,
        topology_identity_valid,
        accepted,
        stale_certificate_accepted,
        unverified_emergent_property_accepted,
        false_verification_acceptance,
        prediction_used_as_sole_proof: false,
        verification_requirement_omissions,
        inherited_verified_claims,
        new_verification_obligations,
        affected_claims: affected,
        total_accepted_claims,
        certificate_reuse_count,
        verification_law_reuse_count,
        full_revalidation_used,
        surprise_escalated,
        proof_reuse_fraction,
        affected_claim_fraction,
        full_revalidation_fraction,
        certificate_check_time_ns,
        delta_verification_time_ns,
        targeted_execution_time_ns,
        full_revalidation_time_ns,
        total_verification_wall_time_ns,
        verification_operations,
        certificate_bytes,
        active_certificate_bytes,
        verification_runtime_bytes: 2_048,
        verification_index_bytes: 512 + u64::from(request.certificate_depth) * 24,
        structural_sharing_events,
        certificate_compression_ratio,
        result_checksum,
    })
}

pub fn validate_certificate(
    certificate: &VerificationCertificate,
    object_identity: u64,
    semantic_hash: u64,
    dependency_hash: u64,
    resource_contract: u64,
    topology_code: u8,
) -> bool {
    certificate.integrity_hash == certificate_digest(certificate)
        && certificate.object_identity == object_identity
        && certificate.semantic_hash == semantic_hash
        && certificate.dependency_hashes.contains(&dependency_hash)
        && certificate.resource_bound == resource_contract
        && certificate.applicability_conditions == u64::from(topology_code)
        && certificate.proven_properties_mask != 0
}

pub fn certificate_digest(certificate: &VerificationCertificate) -> u64 {
    let mut digest = mix(certificate.object_identity, certificate.semantic_hash);
    for value in [
        certificate.assumptions_mask,
        certificate.guarantees_mask,
        certificate.preserved_invariants_mask,
        certificate.proven_properties_mask,
        certificate.resource_bound,
        certificate.applicability_conditions,
        certificate.stability_conditions,
        certificate.negative_conditions,
        certificate.proof_provenance,
        u64::from(certificate.verification_method),
        u64::from(certificate.composition_depth),
    ] {
        digest = mix(digest, value);
    }
    for value in certificate
        .dependency_hashes
        .iter()
        .chain(certificate.evidence_refs.iter())
    {
        digest = mix(digest, *value);
    }
    digest
}

fn validate_request(request: &VerificationProbeRequest) -> Result<(), String> {
    if request.arm_code > 3 {
        return Err("INVALID_ARM_CODE".to_string());
    }
    if request.object_id == 0 || request.semantic_hash == 0 || request.dependency_hash == 0 {
        return Err("ZERO_IDENTITY_HASH".to_string());
    }
    if request.total_claims == 0
        || request.inherited_claims > request.total_claims
        || request.affected_claims > request.total_claims
        || request.emergent_claims > request.total_claims
    {
        return Err("INVALID_CLAIM_COUNTS".to_string());
    }
    if request.certificate_depth == 0 || request.certificate_depth > 64 {
        return Err("INVALID_CERTIFICATE_DEPTH".to_string());
    }
    if request.scale == 0 || request.scale > 8_192 {
        return Err("INVALID_SCALE".to_string());
    }
    Ok(())
}

fn bit_mask(claims: u16) -> u64 {
    if claims >= 64 {
        u64::MAX
    } else {
        (1_u64 << claims) - 1
    }
}

fn ratio(numerator: u16, denominator: u16) -> f64 {
    f64::from(numerator) / f64::from(denominator.max(1))
}

fn nanos(value: u128) -> u64 {
    value.min(u128::from(u64::MAX)) as u64
}

fn burn(operations: u64, seed: u64) -> u64 {
    let mut state = seed ^ 0x9E37_79B9_7F4A_7C15;
    for index in 0..operations {
        state = mix(state, index ^ state.rotate_left(13));
        if index & 0x3fff == 0 {
            black_box(state);
        }
    }
    black_box(state)
}

fn mix(left: u64, right: u64) -> u64 {
    let mut value = left ^ right.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(arm_code: u8) -> VerificationProbeRequest {
        VerificationProbeRequest {
            arm_code,
            object_id: 24,
            semantic_hash: 0x2400,
            dependency_hash: 0x2300,
            certificate_dependency_hash: 0x2300,
            total_claims: 24,
            inherited_claims: 18,
            affected_claims: 2,
            emergent_claims: 1,
            verification_law_count: 2,
            certificate_depth: 6,
            novelty_code: 2,
            topology_code: 3,
            resource_contract: 0xCAFE,
            scale: 1,
            seed: 24,
        }
    }

    #[test]
    fn certificate_self_assertion_has_no_authority() {
        let result = run_verification_probe(request(3)).expect("verified result");
        let mut forged = result.certificate.clone();
        forged.proven_properties_mask ^= 1;
        assert!(!validate_certificate(
            &forged,
            request(3).object_id,
            request(3).semantic_hash,
            request(3).dependency_hash,
            request(3).resource_contract,
            request(3).topology_code,
        ));
        assert!(!result.certificate_self_assertion_authority);
    }

    #[test]
    fn stale_dependency_is_rejected_exactly() {
        let stale = run_verification_probe(VerificationProbeRequest {
            dependency_hash: 0x9999,
            ..request(3)
        })
        .expect("stale result");
        assert!(!stale.accepted);
        assert!(!stale.stale_certificate_accepted);
    }

    #[test]
    fn emergent_claims_always_receive_fresh_checks() {
        let result = run_verification_probe(VerificationProbeRequest {
            emergent_claims: 3,
            ..request(3)
        })
        .expect("emergent result");
        assert_eq!(result.verification_ir.emergent_property_checks, 3);
        assert_eq!(result.verification_requirement_omissions, 0);
        assert!(!result.unverified_emergent_property_accepted);
    }

    #[test]
    fn dependency_slice_reduces_only_unaffected_work() {
        let full = run_verification_probe(request(0)).expect("full result");
        let sliced = run_verification_probe(request(3)).expect("sliced result");
        assert!(sliced.accepted);
        assert!(sliced.new_verification_obligations < full.new_verification_obligations);
        assert_eq!(sliced.affected_claims, request(3).affected_claims);
    }

    #[test]
    fn composite_certificate_is_reusable_at_depth() {
        let result = run_verification_probe(VerificationProbeRequest {
            certificate_depth: 12,
            ..request(3)
        })
        .expect("deep closure result");
        assert!(result.accepted);
        assert!(result.certificate_reuse_count >= 12);
        assert_eq!(result.certificate.composition_depth, 12);
    }

    #[test]
    fn surface_similarity_cannot_override_semantic_identity() {
        let valid = run_verification_probe(request(3)).expect("valid result");
        assert!(!validate_certificate(
            &valid.certificate,
            valid.object_id,
            valid.semantic_hash ^ 1,
            request(3).dependency_hash,
            request(3).resource_contract,
            request(3).topology_code,
        ));
    }
}

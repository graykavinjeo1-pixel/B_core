"""Fail-closed promotion policy for replacing the temporary OCR teacher."""

from __future__ import annotations

from dataclasses import asdict, dataclass
from enum import Enum


class OcrStage(str, Enum):
    BOOTSTRAP = "bootstrap"
    SHADOW = "shadow"
    CANARY = "canary"
    PRIMARY_WITH_FALLBACK = "primary_with_fallback"
    INDEPENDENT = "independent"


@dataclass(frozen=True)
class PromotionMetrics:
    verified_line_crops: int
    blind_documents: int
    blind_template_families: int
    character_accuracy: float
    critical_field_exact: float
    table_structure_f1: float
    numeric_error_rate: float
    native_p95_ms: float
    teacher_p95_ms: float
    consecutive_passing_runs: int
    source_specific_rules: int = 0
    invalid_unicode_outputs: int = 0
    text_detection_recall: float = 1.0
    text_detection_precision: float = 1.0
    authoritative_evidence_coverage: float = 1.0
    authoritative_false_positive_rate: float = 0.0
    external_ocr_invocations: int = 0


@dataclass(frozen=True)
class SpecialistPromotionMetrics:
    independent_documents: int
    independent_template_families: int
    character_accuracy_delta: float
    line_exact_delta: float
    critical_field_exact_delta: float
    router_false_positive_rate: float
    consecutive_passing_runs: int
    source_specific_rules: int = 0
    invalid_unicode_outputs: int = 0


@dataclass(frozen=True)
class GlobalReplacementMetrics:
    independent_documents: int
    independent_template_families: int
    aggregate_character_accuracy_delta: float
    aggregate_line_exact_delta: float
    aggregate_critical_field_exact_delta: float
    worst_document_character_accuracy_delta: float
    worst_document_line_exact_delta: float
    worst_document_critical_field_exact_delta: float
    aggregate_aligned_edits_delta: int = 0
    improved_template_families: int = 0
    regressed_template_families: int = 0
    source_specific_rules: int = 0
    invalid_unicode_outputs: int = 0


@dataclass(frozen=True)
class SparseSpecialistMetrics:
    independent_documents: int
    independent_template_families: int
    character_accuracy_delta: float
    line_exact_delta: float
    critical_field_exact_delta: float
    router_false_positive_rate: float
    consecutive_passing_runs: int
    end_to_end_raster_evaluated: bool = False
    end_to_end_visual_adjudication_samples: int = 0
    runtime_latency_evaluated: bool = False
    runtime_latency_ratio: float | None = None
    source_specific_rules: int = 0
    invalid_unicode_outputs: int = 0


@dataclass(frozen=True)
class SparseSpecialistResult:
    evidence_passed: bool
    production_ready: bool
    permitted_mode: str
    evidence_reasons: tuple[str, ...]
    production_blockers: tuple[str, ...]
    metrics: SparseSpecialistMetrics

    def as_dict(self) -> dict:
        return {
            "schema": "B_CORE_NATIVE_OCR_SPARSE_SPECIALIST_GATE_1",
            "evidencePassed": self.evidence_passed,
            "productionReady": self.production_ready,
            "permittedMode": self.permitted_mode,
            "evidenceReasons": list(self.evidence_reasons),
            "productionBlockers": list(self.production_blockers),
            "metrics": asdict(self.metrics),
        }


@dataclass(frozen=True)
class GlobalReplacementResult:
    passed: bool
    reasons: tuple[str, ...]
    metrics: GlobalReplacementMetrics

    def as_dict(self) -> dict:
        return {
            "schema": "B_CORE_NATIVE_OCR_GLOBAL_REPLACEMENT_GATE_1",
            "passed": self.passed,
            "reasons": list(self.reasons),
            "metrics": asdict(self.metrics),
        }


@dataclass(frozen=True)
class SpecialistPromotionResult:
    passed: bool
    permitted_mode: str
    reasons: tuple[str, ...]
    metrics: SpecialistPromotionMetrics

    def as_dict(self) -> dict:
        return {
            "schema": "B_CORE_NATIVE_OCR_SPECIALIST_PROMOTION_1",
            "passed": self.passed,
            "permittedMode": self.permitted_mode,
            "reasons": list(self.reasons),
            "metrics": asdict(self.metrics),
        }


@dataclass(frozen=True)
class PromotionResult:
    current_stage: OcrStage
    permitted_stage: OcrStage
    passed: bool
    reasons: tuple[str, ...]
    metrics: PromotionMetrics

    def as_dict(self) -> dict:
        return {
            "schema": "B_CORE_NATIVE_OCR_PROMOTION_1",
            "currentStage": self.current_stage.value,
            "permittedStage": self.permitted_stage.value,
            "passed": self.passed,
            "reasons": list(self.reasons),
            "metrics": asdict(self.metrics),
        }


def evaluate_promotion(current_stage: OcrStage, metrics: PromotionMetrics) -> PromotionResult:
    """Return the highest safe next stage; never skip a stage.

    Accuracy is measured on documents excluded by document hash before crop
    generation.  Development pages and lines from the same PDF cannot satisfy
    these gates.
    """
    reasons: list[str] = []
    if metrics.source_specific_rules != 0:
        reasons.append("SOURCE_SPECIFIC_RULES_PRESENT")
    if metrics.invalid_unicode_outputs != 0:
        reasons.append("INVALID_UNICODE_OUTPUT")

    target = current_stage
    if current_stage == OcrStage.BOOTSTRAP:
        if metrics.verified_line_crops < 50_000:
            reasons.append("VERIFIED_CORPUS_BELOW_50000")
        if metrics.blind_documents < 40 or metrics.blind_template_families < 8:
            reasons.append("BLIND_COVERAGE_INSUFFICIENT")
        if metrics.character_accuracy < 0.970:
            reasons.append("CHARACTER_ACCURACY_BELOW_SHADOW_GATE")
        if not reasons:
            target = OcrStage.SHADOW
    elif current_stage == OcrStage.SHADOW:
        if metrics.blind_documents < 100 or metrics.blind_template_families < 15:
            reasons.append("BLIND_COVERAGE_INSUFFICIENT")
        if metrics.critical_field_exact < 0.985:
            reasons.append("CRITICAL_FIELD_EXACT_BELOW_CANARY_GATE")
        if metrics.table_structure_f1 < 0.970:
            reasons.append("TABLE_STRUCTURE_BELOW_CANARY_GATE")
        if metrics.numeric_error_rate > 0.002:
            reasons.append("NUMERIC_ERROR_RATE_ABOVE_CANARY_GATE")
        if metrics.consecutive_passing_runs < 2:
            reasons.append("NEEDS_TWO_CONSECUTIVE_BLIND_PASSES")
        if not reasons:
            target = OcrStage.CANARY
    elif current_stage == OcrStage.CANARY:
        if metrics.blind_documents < 200 or metrics.blind_template_families < 20:
            reasons.append("BLIND_COVERAGE_INSUFFICIENT")
        if metrics.critical_field_exact < 0.993:
            reasons.append("CRITICAL_FIELD_EXACT_BELOW_PRIMARY_GATE")
        if metrics.table_structure_f1 < 0.985:
            reasons.append("TABLE_STRUCTURE_BELOW_PRIMARY_GATE")
        if metrics.numeric_error_rate > 0.001:
            reasons.append("NUMERIC_ERROR_RATE_ABOVE_PRIMARY_GATE")
        if metrics.native_p95_ms > metrics.teacher_p95_ms:
            reasons.append("NATIVE_P95_NOT_FASTER")
        if metrics.consecutive_passing_runs < 3:
            reasons.append("NEEDS_THREE_CONSECUTIVE_BLIND_PASSES")
        if not reasons:
            target = OcrStage.PRIMARY_WITH_FALLBACK
    elif current_stage == OcrStage.PRIMARY_WITH_FALLBACK:
        if metrics.blind_documents < 500 or metrics.blind_template_families < 40:
            reasons.append("INDEPENDENCE_COVERAGE_INSUFFICIENT")
        if metrics.character_accuracy < 0.997:
            reasons.append("CHARACTER_ACCURACY_BELOW_INDEPENDENCE_GATE")
        if metrics.critical_field_exact < 0.997:
            reasons.append("CRITICAL_FIELD_EXACT_BELOW_INDEPENDENCE_GATE")
        if metrics.table_structure_f1 < 0.993:
            reasons.append("TABLE_STRUCTURE_BELOW_INDEPENDENCE_GATE")
        if metrics.numeric_error_rate > 0.0005:
            reasons.append("NUMERIC_ERROR_RATE_ABOVE_INDEPENDENCE_GATE")
        if metrics.text_detection_recall < 0.995:
            reasons.append("TEXT_DETECTION_RECALL_BELOW_INDEPENDENCE_GATE")
        if metrics.text_detection_precision < 0.995:
            reasons.append("TEXT_DETECTION_PRECISION_BELOW_INDEPENDENCE_GATE")
        if metrics.authoritative_evidence_coverage < 0.980:
            reasons.append("AUTHORITATIVE_COVERAGE_BELOW_INDEPENDENCE_GATE")
        if metrics.authoritative_false_positive_rate != 0:
            reasons.append("AUTHORITATIVE_FALSE_POSITIVE_PRESENT")
        if metrics.external_ocr_invocations != 0:
            reasons.append("EXTERNAL_OCR_INVOCATION_PRESENT")
        if metrics.consecutive_passing_runs < 5:
            reasons.append("NEEDS_FIVE_CONSECUTIVE_BLIND_PASSES")
        if not reasons:
            target = OcrStage.INDEPENDENT

    return PromotionResult(
        current_stage=current_stage,
        permitted_stage=target,
        passed=target != current_stage or current_stage == OcrStage.INDEPENDENT,
        reasons=tuple(reasons),
        metrics=metrics,
    )


def evaluate_table_cell_specialist(metrics: SpecialistPromotionMetrics) -> SpecialistPromotionResult:
    """Fail closed before routing a narrow recognizer on production table cells."""

    reasons: list[str] = []
    if metrics.source_specific_rules:
        reasons.append("SOURCE_SPECIFIC_RULES_PRESENT")
    if metrics.invalid_unicode_outputs:
        reasons.append("INVALID_UNICODE_OUTPUT")
    if metrics.independent_documents < 12 or metrics.independent_template_families < 6:
        reasons.append("INDEPENDENT_TABLE_COVERAGE_INSUFFICIENT")
    if metrics.character_accuracy_delta <= 0:
        reasons.append("NO_CHARACTER_ACCURACY_GAIN")
    if metrics.line_exact_delta < 0:
        reasons.append("LINE_EXACT_REGRESSION")
    if metrics.critical_field_exact_delta < 0:
        reasons.append("CRITICAL_FIELD_REGRESSION")
    if metrics.router_false_positive_rate > 0.005:
        reasons.append("ROUTER_FALSE_POSITIVE_RATE_ABOVE_GATE")
    if metrics.consecutive_passing_runs < 3:
        reasons.append("NEEDS_THREE_CONSECUTIVE_BLIND_PASSES")
    passed = not reasons
    return SpecialistPromotionResult(
        passed=passed,
        permitted_mode="table_cell_primary" if passed else "research_shadow_only",
        reasons=tuple(reasons),
        metrics=metrics,
    )


def evaluate_global_replacement(metrics: GlobalReplacementMetrics) -> GlobalReplacementResult:
    """Reject an average improvement that hides damage to one document family."""

    reasons: list[str] = []
    if metrics.source_specific_rules:
        reasons.append("SOURCE_SPECIFIC_RULES_PRESENT")
    if metrics.invalid_unicode_outputs:
        reasons.append("INVALID_UNICODE_OUTPUT")
    if metrics.independent_documents < 12 or metrics.independent_template_families < 6:
        reasons.append("INDEPENDENT_COVERAGE_INSUFFICIENT")
    if metrics.aggregate_character_accuracy_delta <= 0:
        reasons.append("NO_AGGREGATE_CHARACTER_GAIN")
    if metrics.aggregate_line_exact_delta < 0:
        reasons.append("AGGREGATE_LINE_REGRESSION")
    if metrics.aggregate_critical_field_exact_delta < 0:
        reasons.append("AGGREGATE_CRITICAL_REGRESSION")
    if metrics.aggregate_aligned_edits_delta > 0:
        reasons.append("AGGREGATE_ALIGNED_EDIT_REGRESSION")
    if metrics.regressed_template_families:
        reasons.append("TEMPLATE_FAMILY_ALIGNED_EDIT_REGRESSION")
    if metrics.improved_template_families < 3:
        reasons.append("DIVERSE_TEMPLATE_GAIN_INSUFFICIENT")
    if metrics.worst_document_character_accuracy_delta < -0.001:
        reasons.append("DOCUMENT_CHARACTER_REGRESSION")
    if metrics.worst_document_line_exact_delta < 0:
        reasons.append("DOCUMENT_LINE_REGRESSION")
    if metrics.worst_document_critical_field_exact_delta < 0:
        reasons.append("DOCUMENT_CRITICAL_REGRESSION")
    return GlobalReplacementResult(passed=not reasons, reasons=tuple(reasons), metrics=metrics)


def evaluate_sparse_specialist(metrics: SparseSpecialistMetrics) -> SparseSpecialistResult:
    """Separate generalization evidence from authority to enter production.

    A target-free specialist may earn research-shadow status from independent
    line crops, but it cannot enter a runtime canary until the complete raster
    path and real dual-model latency have also been measured.  This prevents a
    benchmark win from silently becoming production authority.
    """

    evidence_reasons: list[str] = []
    if metrics.source_specific_rules:
        evidence_reasons.append("SOURCE_SPECIFIC_RULES_PRESENT")
    if metrics.invalid_unicode_outputs:
        evidence_reasons.append("INVALID_UNICODE_OUTPUT")
    if metrics.independent_documents < 12 or metrics.independent_template_families < 6:
        evidence_reasons.append("INDEPENDENT_COVERAGE_INSUFFICIENT")
    if metrics.character_accuracy_delta <= 0:
        evidence_reasons.append("NO_CHARACTER_ACCURACY_GAIN")
    if metrics.line_exact_delta < 0:
        evidence_reasons.append("LINE_EXACT_REGRESSION")
    if metrics.critical_field_exact_delta < 0:
        evidence_reasons.append("CRITICAL_FIELD_REGRESSION")
    if metrics.router_false_positive_rate > 0.005:
        evidence_reasons.append("ROUTER_FALSE_POSITIVE_RATE_ABOVE_GATE")
    if metrics.consecutive_passing_runs < 3:
        evidence_reasons.append("NEEDS_THREE_CONSECUTIVE_BLIND_PASSES")

    evidence_passed = not evidence_reasons
    production_blockers: list[str] = []
    if not metrics.end_to_end_raster_evaluated:
        production_blockers.append("END_TO_END_RASTER_NOT_EVALUATED")
    if metrics.end_to_end_visual_adjudication_samples < 50:
        production_blockers.append("END_TO_END_VISUAL_ADJUDICATION_INSUFFICIENT")
    if not metrics.runtime_latency_evaluated:
        production_blockers.append("RUNTIME_LATENCY_NOT_EVALUATED")
    elif metrics.runtime_latency_ratio is None or metrics.runtime_latency_ratio > 1.15:
        production_blockers.append("RUNTIME_LATENCY_OVERHEAD_ABOVE_GATE")
    production_ready = evidence_passed and not production_blockers
    if production_ready:
        permitted_mode = "canary"
    elif evidence_passed:
        permitted_mode = "research_shadow_only"
    else:
        permitted_mode = "rejected"
    return SparseSpecialistResult(
        evidence_passed=evidence_passed,
        production_ready=production_ready,
        permitted_mode=permitted_mode,
        evidence_reasons=tuple(evidence_reasons),
        production_blockers=tuple(production_blockers),
        metrics=metrics,
    )

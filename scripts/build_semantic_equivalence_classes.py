"""Build offline semantic-equivalence classes from inverse-approved evidence.

Surfaces are evidence only.  The generated artifact is explicitly barred from
runtime template installation and expression-primitive promotion.
"""
from __future__ import annotations

import argparse, hashlib, json, re
from collections import Counter, defaultdict
from pathlib import Path

SOURCES = (
    ("existing_neutral", r"D:\B_Core_validation\expression_primitive_learning_v1\expression_primitive_candidates_ko_v1.json", r"D:\B_Core_validation\expression_primitive_learning_v1\expression_primitive_gated_ko_v1.json"),
    ("runtime_composition", r"D:\B_Core_validation\expression_rewrite_calibration_v1\recovered_runtime_anchor_candidates_ko_v1.json", r"D:\B_Core_validation\expression_rewrite_calibration_v1\recovered_runtime_anchor_gated_ko_v1.json"),
    ("independent_state", r"D:\B_Core_validation\natural_state_language_grounding_v1\natural_state_language_grounding_candidates_ko_v2.json", r"D:\B_Core_validation\natural_state_language_grounding_v1\natural_state_language_grounding_gated_ko_v2.json"),
    ("paired_state", r"D:\B_Core_validation\multi_factor_language_state_v1\paired_teacher_state_sensitivity_candidates_ko_v1.json", r"D:\B_Core_validation\multi_factor_language_state_v1\paired_teacher_state_sensitivity_gated_ko_v1.json"),
    ("speech_act_coverage", r"D:\B_Core_validation\multi_factor_language_state_v1\speech_act_state_relevance_candidates_ko_v1.json", r"D:\B_Core_validation\multi_factor_language_state_v1\speech_act_state_relevance_gated_ko_v1.json"),
    ("targeted_structural_gap", r"D:\B_Core_validation\expression_rewrite_calibration_v1\structural_gap_anchor_candidates_ko_v1.json", r"D:\B_Core_validation\expression_rewrite_calibration_v1\structural_gap_anchor_gated_ko_v1.json"),
    ("approved_targeted_rewrite", r"D:\B_Core_validation\expression_rewrite_calibration_v1\primitive_specific_rewrite_candidates_ko_v1.json", r"D:\B_Core_validation\expression_rewrite_calibration_v1\primitive_specific_rewrite_gated_ko_v1.json"),
)

def digest(value):
    return hashlib.sha256(json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()).hexdigest()

def semantic_split(canonical_hash: str) -> str:
    bucket = int(canonical_hash[:8], 16) % 100
    return "TRAIN" if bucket < 70 else "VALIDATION" if bucket < 85 else "BLIND"

def slots(ir: dict) -> list[tuple[str, str]]:
    pairs=[]
    for i, claim in enumerate(ir.get("claims", [])):
        subject=claim.get("subject", {})
        if subject.get("canonical_lexical_label"): pairs.append((subject["canonical_lexical_label"], f"<SUBJECT_{i}>"))
        value=claim.get("value", {})
        payload=value.get("value", {})
        if value.get("kind") == "LEXICAL" and payload.get("canonical_lexical_label"):
            pairs.append((payload["canonical_lexical_label"], f"<VALUE_{i}>"))
        if value.get("kind") == "CLOCK":
            h,m=payload.get("hour"),payload.get("minute")
            if isinstance(h,int) and isinstance(m,int):
                ap="오전" if h < 12 else "오후"; hh=h if 1 <= h <= 12 else h-12
                for text in (f"{h:02d}:{m:02d}", f"{h}시 {m}분", f"{ap} {hh}시 {m}분", f"{ap} {hh}시"):
                    pairs.append((text, f"<TIME_{i}>"))
    return sorted(pairs, key=lambda item: len(item[0]), reverse=True)

def normalized(surface: str, ir: dict) -> str:
    value=surface
    for text, token in slots(ir):
        value=value.replace(text, token)
    return value

def main():
    ap=argparse.ArgumentParser(); ap.add_argument("--output",type=Path,required=True); args=ap.parse_args()
    classes=defaultdict(dict); excluded=Counter()
    for name,candidate_name,gate_name in SOURCES:
        cp,gp=Path(candidate_name),Path(gate_name)
        if not cp.exists() or not gp.exists(): excluded[f"{name}:MISSING"]+=1; continue
        candidates=json.loads(cp.read_text(encoding="utf-8")).get("rows",[])
        approved={x["prompt_sha256"] for x in json.loads(gp.read_text(encoding="utf-8")) if x.get("approval")=="APPROVED_STATE_CONTRAST_GOLD"}
        for row in candidates:
            if row.get("prompt_sha256") not in approved: continue
            ir=row.get("approved_response_ir"); surface=row.get("surface")
            if not ir or not surface: excluded[f"{name}:MISSING_IR_OR_SURFACE"]+=1; continue
            canonical=digest(ir); key=(canonical, hashlib.sha256(surface.encode()).hexdigest())
            classes[canonical].setdefault(key, {"surface":surface,"surface_sha256":key[1],"normalized_surface":normalized(surface,ir),"provenances":[],"approved_response_ir":ir})["provenances"].append({"source":name,"prompt_sha256":row["prompt_sha256"],"candidate_path":str(cp),"gate_path":str(gp)})
    output=[]
    for canonical, surfaces in sorted(classes.items()):
        values=list(surfaces.values()); ir=values[0]["approved_response_ir"]
        output.append({"canonical_hash":canonical,"meaning_family_id":ir.get("semantic_sha256",canonical),"split":semantic_split(canonical),"speech_act":ir.get("speech_act"),"approved_response_ir":ir,"surface_count":len(values),"surfaces":values})
    artifact={"schema":"BCORE.SEMANTIC_EQUIVALENCE_CLASSES.V1","acquisition":"INVERSE_APPROVED_EVIDENCE_ONLY","runtime_installation":"FORBIDDEN","expression_primitive_promotion":"FORBIDDEN","class_count":len(output),"classes_with_variation":sum(x["surface_count"]>1 for x in output),"surface_count":sum(x["surface_count"] for x in output),"excluded":dict(excluded),"classes":output,"artifact_sha256":""}
    artifact["artifact_sha256"]=digest(artifact); args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(artifact,ensure_ascii=False,indent=2),encoding="utf-8")
    print(json.dumps({"classes":artifact["class_count"],"with_variation":artifact["classes_with_variation"],"surfaces":artifact["surface_count"],"output":str(args.output)},ensure_ascii=False))
if __name__=='__main__': main()

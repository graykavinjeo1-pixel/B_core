"""Discover recurring structural deltas inside trusted semantic equivalence classes."""
from __future__ import annotations

import argparse, hashlib, itertools, json, re
from collections import Counter, defaultdict
from pathlib import Path

from analyze_paired_teacher_state_sensitivity import features


def particle_map(text: str) -> tuple[str, ...]:
    return tuple(sorted(re.findall(r"<(?:SUBJECT|VALUE)_\d+>(은|는|이|가|을|를|으로|로|에|에서)", text)))

def ending(text: str) -> str:
    return str(features(text)["ending_family"])

def structural_signature(left: str, right: str) -> tuple[str, ...]:
    # Pair ordering is arbitrary for discovery.  Record invariant replacement
    # types, not an instruction-shaped direction.
    parts=[]
    le,re_=ending(left),ending(right)
    if le != re_: parts.append("ending:{"+"|".join(sorted((le,re_)))+"}")
    lp,rp=particle_map(left),particle_map(right)
    if lp != rp: parts.append("particle:{"+"|".join(sorted(("/".join(lp) or "NONE","/".join(rp) or "NONE")))+"}")
    lf,rf=features(left),features(right)
    for key in ("honorific","deference","hedge","discourse_marker","emotion_marker","subject_omitted","information_order"):
        if lf[key] != rf[key]: parts.append(f"{key}:{{{lf[key]}|{rf[key]}}}")
    for key in ("clause_count","token_count","char_count"):
        if lf[key] != rf[key]: parts.append(f"{key}:CHANGE")
    left_tokens=set(re.findall(r"[가-힣A-Za-z]+",left)); right_tokens=set(re.findall(r"[가-힣A-Za-z]+",right))
    if left_tokens != right_tokens: parts.append("lexical_substitution")
    if not parts: return ("IDENTICAL_NORMALIZED",)
    return tuple(sorted(parts))


def classify(signature: tuple[str,...], family_splits: dict[str,set[str]]) -> str:
    family_count=len(family_splits); all_splits=all(any(split in seen for seen in family_splits.values()) for split in ("TRAIN","VALIDATION","BLIND"))
    if family_count < 2: return "INSUFFICIENT_RECURRENCE"
    if not all_splits: return "UNSTABLE"
    if signature == ("lexical_substitution",): return "LEXICAL_VARIANT"
    if all(item.startswith(("ending:","particle:","honorific:","deference:")) for item in signature): return "MORPHOLOGICAL_VARIANT"
    if family_count >= 3: return "REUSABLE_PRIMITIVE" if len(signature)==1 else "COMPOSITE_PRIMITIVE"
    return "CONSTRUCTION_SPECIFIC"


def main():
    ap=argparse.ArgumentParser();ap.add_argument("--classes",type=Path,required=True);ap.add_argument("--output",type=Path,required=True);args=ap.parse_args()
    data=json.loads(args.classes.read_text(encoding="utf-8")); pairs=[]; clusters=defaultdict(lambda:{"families":defaultdict(set),"pairs":[]})
    for equivalence in data["classes"]:
        surfaces=equivalence["surfaces"]
        for left,right in itertools.combinations(surfaces,2):
            signature=structural_signature(left["normalized_surface"],right["normalized_surface"])
            source_artifacts=sorted({
                item["source"] for item in left["provenances"] + right["provenances"]
            })
            row={"canonical_hash":equivalence["canonical_hash"],"meaning_family_id":equivalence["meaning_family_id"],"split":equivalence["split"],"speech_act":equivalence["speech_act"],"left_surface_sha256":left["surface_sha256"],"right_surface_sha256":right["surface_sha256"],"source_artifacts":source_artifacts,"signature":signature}
            pairs.append(row);clusters[signature]["families"][equivalence["meaning_family_id"]].add(equivalence["split"]);clusters[signature]["pairs"].append(row)
    cluster_rows=[]
    for signature,record in sorted(clusters.items(),key=lambda item:(-len(item[1]["families"]),item[0])):
        family_splits={k:sorted(v) for k,v in record["families"].items()}
        source_counts=Counter(source for pair in record["pairs"] for source in pair["source_artifacts"])
        cluster_rows.append({"signature":signature,"family_count":len(family_splits),"split_family_counts":{s:sum(s in x for x in family_splits.values()) for s in ("TRAIN","VALIDATION","BLIND")},"classification":classify(signature,record["families"]),"family_ids":sorted(family_splits),"pair_count":len(record["pairs"]),"source_artifact_pair_counts":dict(source_counts)})
    artifact={"schema":"BCORE.SEMANTIC_EQUIVALENCE_EXPRESSION_MINING_REPORT.V1","classes_sha256":hashlib.sha256(args.classes.read_bytes()).hexdigest(),"surface_authority":"INVERSE_APPROVED_EQUIVALENCE_EVIDENCE_ONLY","slot_normalization":"CANONICAL_ENTITY_VALUE_TIME_SLOTS","runtime_installation":"FORBIDDEN","expression_primitive_promotion":"FORBIDDEN","equivalence_pair_count":len(pairs),"cluster_count":len(cluster_rows),"classification_counts":dict(Counter(x["classification"] for x in cluster_rows)),"clusters":cluster_rows,"pairs":pairs,"artifact_sha256":""}
    artifact["artifact_sha256"]=hashlib.sha256(json.dumps(artifact,ensure_ascii=False,sort_keys=True,separators=(",",":")).encode()).hexdigest();args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(artifact,ensure_ascii=False,indent=2),encoding="utf-8")
    print(json.dumps({"pairs":len(pairs),"clusters":len(cluster_rows),"classifications":artifact["classification_counts"],"output":str(args.output)},ensure_ascii=False))
if __name__=='__main__': main()

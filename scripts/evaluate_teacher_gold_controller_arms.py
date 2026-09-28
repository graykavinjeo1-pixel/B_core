"""Evaluate frozen OFF/ON controller against independently teacher-gated pairs."""
from __future__ import annotations
import argparse, hashlib, json, re
from collections import defaultdict
from pathlib import Path
import numpy as np

MARKERS = (("바로 말하면", "핵심은", "요점을 정리하면", "정리해 보면", "먼저 말하면"), ("괜찮", "다행", "함께", "걱정", "안심"), ("아이구", "참", "정말", "좋아", "답답", "속상"), ("아마", "것 같", "가능", "대체로", "아마도"), ("먼저", "정리하면", "요점을", "참고로", "결론적으로", "그러니까"))

def vec(text: str) -> np.ndarray:
    tail=text.rstrip(); ending="OTHER"
    for name,suffixes in {"FORMAL":("습니다","습니까"),"POLITE":("어요","아요","해요"),"INFORMAL":("야","어","아","해"),"PLAIN":("다",),"ELDER":("구먼",)}.items():
        if any(tail.endswith(s+p) for s in suffixes for p in ("",".","!","?")): ending=name; break
    values=[len(text),len(text.split()),len(re.findall(r"[.!?。！？]",text)),int("\n" in text),int(any(m in text for m in ("#","- ","1.")))]
    values.extend(int(any(m in text for m in group)) for group in MARKERS)
    values.append(int(bool(re.search(r"^[^,，]+(?:이|가|은|는)",text))))
    values.extend(int(ending==name) for name in ("FORMAL","POLITE","INFORMAL","PLAIN","ELDER","OTHER"))
    return np.asarray(values,dtype=float)

def cosine(a,b):
    d=float(np.linalg.norm(a)*np.linalg.norm(b)); return float(np.dot(a,b)/d) if d else 1.0

def reports(paths):
    out={}
    for p in paths:
        d=json.loads(p.read_text(encoding="utf-8")); out[d["source_response_sha256"]]=d
    return out

def main():
    ap=argparse.ArgumentParser(); ap.add_argument("--gold",type=Path,required=True); ap.add_argument("--off",nargs="+",type=Path,required=True); ap.add_argument("--on",nargs="+",type=Path,required=True); ap.add_argument("--output",type=Path,required=True); args=ap.parse_args()
    gold=json.loads(args.gold.read_text(encoding="utf-8")); off=reports(args.off); on=reports(args.on)
    factor_map={"age_register":"age_register_condition","korean_dialect":"region_dialect_condition"}
    value_map={("age_register","ADULT"):"BASELINE",("age_register","ELDER"):"ELDER",("korean_dialect","STANDARD"):"STANDARD",("korean_dialect","GYEONGSANG"):"GYEONGSANG",("inferred_user_emotion","NEUTRAL"):"BASELINE",("inferred_user_emotion","ANXIOUS"):"TENSE",("dialogue_context","EVERYDAY"):"BASELINE",("dialogue_context","SERVICE"):"SERVICE"}
    rows=[]
    for pair in gold["pairs"]:
        if pair["split"]!="BLIND": continue
        source=pair["meaning_family_id"]; a=off.get(source); b=on.get(source)
        if not a or not b: continue
        runtime_factor=factor_map.get(pair["factor"],pair["factor"])
        def find(report,value):
            wanted=value_map.get((pair["factor"],value),value)
            for factor in report.get("factors",[]):
                if factor["factor"]!=runtime_factor: continue
                for v in factor["variants"]:
                    if v["value"]==wanted: return v["output"]
            return None
        a_left,a_right=find(a,pair["left_value"]),find(a,pair["right_value"]); b_left,b_right=find(b,pair["left_value"]),find(b,pair["right_value"])
        if not all((a_left,a_right,b_left,b_right)): continue
        gd=vec(pair["right_surface"])-vec(pair["left_surface"]); ad=vec(a_right["markdown"])-vec(a_left["markdown"]); bd=vec(b_right["markdown"])-vec(b_left["markdown"])
        rows.append({"meaning_family_id":source,"factor":pair["factor"],"gold_pair_changed":pair["surface_pair_changed"],"arm_a_pair_changed":a_left["markdown"]!=a_right["markdown"],"arm_b_pair_changed":b_left["markdown"]!=b_right["markdown"],"arm_a_style_cosine":cosine(ad,gd),"arm_b_style_cosine":cosine(bd,gd),"arm_a_semantic_inverse":bool(a_left["semantic_inverse"] and a_right["semantic_inverse"]),"arm_b_semantic_inverse":bool(b_left["semantic_inverse"] and b_right["semantic_inverse"])})
    summary={}
    for factor in sorted({r["factor"] for r in rows}):
        subset=[r for r in rows if r["factor"]==factor]
        summary[factor]={"n":len(subset),"gold_pair_changed_rate":float(np.mean([r["gold_pair_changed"] for r in subset])),"arm_a_pair_changed_rate":float(np.mean([r["arm_a_pair_changed"] for r in subset])),"arm_b_pair_changed_rate":float(np.mean([r["arm_b_pair_changed"] for r in subset])),"arm_a_style_cosine":float(np.mean([r["arm_a_style_cosine"] for r in subset])),"arm_b_style_cosine":float(np.mean([r["arm_b_style_cosine"] for r in subset])),"arm_a_semantic_inverse":all(r["arm_a_semantic_inverse"] for r in subset),"arm_b_semantic_inverse":all(r["arm_b_semantic_inverse"] for r in subset)}
    result={"schema":"BCORE.TEACHER_GOLD_CONTROLLER_ARMS.V1","gold_artifact_sha256":gold["artifact_sha256"],"sealed_blind_pairs":len(rows),"factor_summary":summary,"pairs":rows,"arm_c":"TEACHER_APPROVED_PAIR_REFERENCE_ONLY","promotion":"RESEARCH_SHADOW_ONLY","artifact_sha256":""}
    result["artifact_sha256"]=hashlib.sha256(json.dumps(result,ensure_ascii=False,separators=(",",":")).encode()).hexdigest(); args.output.parent.mkdir(parents=True,exist_ok=True); args.output.write_text(json.dumps(result,ensure_ascii=False,indent=2),encoding="utf-8"); print(json.dumps({"output":str(args.output),"pairs":len(rows),"artifact_sha256":result["artifact_sha256"]},ensure_ascii=False,indent=2))

if __name__=="__main__": main()

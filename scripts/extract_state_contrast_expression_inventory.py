"""Extract reusable expression differences from approved teacher pairs.

This inventory contains feature transitions only; teacher sentences remain
research evidence and are never installed as runtime templates.
"""
from __future__ import annotations
import argparse, hashlib, json, re
from collections import Counter, defaultdict
from pathlib import Path

def features(text: str) -> dict[str, int]:
    tail=text.rstrip(); ending="OTHER"
    for name,suffixes in {"FORMAL":("습니다","습니까","입니다","입니까"),"POLITE":("어요","아요","해요"),"INFORMAL":("야","어","아","해"),"PLAIN":("다",),"ELDER":("구먼",)}.items():
        if any(tail.endswith(s+p) for s in suffixes for p in ("",".","!","?")): ending=name; break
    return {"chars":len(text),"tokens":len(text.split()),"sentences":len(re.findall(r"[.!?。！？]",text)),"newline":int("\n" in text),"ending_family":ending,"warmth_marker":int(any(x in text for x in ("괜찮","다행","함께","걱정","안심"))),"hedge_marker":int(any(x in text for x in ("아마","것 같","가능","대체로"))),"discourse_marker":int(any(x in text for x in ("먼저","정리하면","요점을","참고로","결론적으로")))}

def main():
    ap=argparse.ArgumentParser();ap.add_argument("--gold",type=Path,required=True);ap.add_argument("--output",type=Path,required=True);args=ap.parse_args();gold=json.loads(args.gold.read_text(encoding="utf-8")); by=defaultdict(list)
    for pair in gold["pairs"]:
        left,right=features(pair["left_surface"]),features(pair["right_surface"]); by[pair["factor"]].append((pair,left,right))
    inventory=[]
    for factor,items in sorted(by.items()):
        changed=[(p,l,r) for p,l,r in items if p["surface_pair_changed"]]
        endings=Counter((l["ending_family"],r["ending_family"]) for _,l,r in changed)
        numeric={name:sum(r[name]-l[name] for _,l,r in changed)/len(changed) if changed else 0.0 for name in ("chars","tokens","sentences","newline","warmth_marker","hedge_marker","discourse_marker")}
        inventory.append({"factor":factor,"pair_count":len(items),"changed_pair_count":len(changed),"changed_rate":len(changed)/len(items) if items else 0.0,"ending_family_transitions":[{"from":a,"to":b,"count":n} for (a,b),n in sorted(endings.items())],"mean_feature_delta":numeric,"promotion":"CANDIDATE_RESEARCH_ONLY"})
    result={"schema":"BCORE.STATE_CONTRAST_EXPRESSION_INVENTORY.V1","source_gold_sha256":gold["artifact_sha256"],"surface_authority":"FEATURE_TRANSITIONS_ONLY_NO_TEACHER_SURFACE_CACHE","inventory":inventory,"runtime_installation":"NONE","promotion":"RESEARCH_SHADOW_ONLY","artifact_sha256":""};result["artifact_sha256"]=hashlib.sha256(json.dumps(result,ensure_ascii=False,separators=(",",":")).encode()).hexdigest();args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(result,ensure_ascii=False,indent=2),encoding="utf-8");print(json.dumps({"output":str(args.output),"artifact_sha256":result["artifact_sha256"],"inventory_entries":len(inventory)},ensure_ascii=False,indent=2))

if __name__=="__main__":main()

"""Fit a shadow-only 5D controller from approved teacher pair differences.

The basis and runtime architecture remain frozen.  Only factor/value deltas
are re-estimated; no surface text is written to the artifact.
"""
from __future__ import annotations
import argparse, hashlib, json
from collections import defaultdict
from pathlib import Path
import numpy as np

def main():
    ap=argparse.ArgumentParser();ap.add_argument("--gold",type=Path,required=True);ap.add_argument("--latent",type=Path,required=True);ap.add_argument("--base-controller",type=Path,required=True);ap.add_argument("--output",type=Path,required=True);args=ap.parse_args()
    gold=json.loads(args.gold.read_text(encoding="utf8")); latent=json.loads(args.latent.read_text(encoding="utf8")); base=json.loads(args.base_controller.read_text(encoding="utf8")); comp=np.asarray(latent["components"],dtype=float)
    factor_map={"age_register":"age_register_condition","korean_dialect":"region_dialect_condition"}
    samples=defaultdict(list)
    for pair in gold["pairs"]:
        # This trainer consumes structural feature differences only.  The pair
        # artifact deliberately contains no canonical meaning authority beyond
        # the already-gated family identity.
        if not pair["surface_pair_changed"]: continue
        factor=factor_map.get(pair["factor"],pair["factor"]); left,right=pair["left_value"],pair["right_value"]
        # Values that represent the baseline condition are not assigned a
        # learned delta.  For two non-baseline values, retain the right-side
        # transition as the relative target and leave the left side unchanged.
        target=right
        if factor=="inferred_user_emotion" and left=="ANXIOUS": target=left
        if factor=="dialogue_context" and right=="SERVICE": target=right
        if factor=="relationship_social_distance" and right=="PROFESSIONAL": target=right
        if factor=="age_register_condition" and right=="ELDER": target=right
        if factor=="region_dialect_condition": continue
        # Reconstruct the same compact feature vector used by the analysis:
        # chars/tokens are excluded from the residual basis.
        # The analysis stores only pair surfaces, so import the shared feature
        # function locally to keep this artifact independent of runtime text.
        import re
        def v(text):
            tail=text.rstrip(); e="OTHER"
            for n,ss in {"FORMAL":("습니다","습니까","입니다","입니까"),"POLITE":("어요","아요","해요"),"INFORMAL":("야","어","아","해"),"PLAIN":("다",),"ELDER":("구먼",)}.items():
                if any(tail.endswith(s+p) for s in ss for p in ("",".","!","?")): e=n; break
            m=(("바로 말하면","핵심은","요점을 정리하면","정리해 보면","먼저 말하면"),("괜찮","다행","함께","걱정","안심"),("아이구","참","정말","좋아","답답","속상"),("아마","것 같","가능","대체로","아마도"),("먼저","정리하면","요점을","참고로","결론적으로","그러니까"))
            x=[len(text),len(text.split()),len(re.findall(r"[.!?。！？]",text)),int("\n" in text),int(any(a in text for a in ("#","- ","1.")))];x.extend(int(any(a in text for a in g)) for g in m);x.append(int(bool(re.search(r"^[^,，]+(?:이|가|은|는)",text))));x.extend(int(e==n) for n in ("FORMAL","POLITE","INFORMAL","PLAIN","ELDER","OTHER"));return np.asarray(x,float)
        delta=v(pair["right_surface"])-v(pair["left_surface"]); verbosity_signal=float(delta[0]*80.0 + delta[1]*160.0); delta[:2]=0; proj=delta@comp.T
        if target==left: proj=-proj; verbosity_signal=-verbosity_signal
        samples[(factor,target)].append(np.concatenate(([verbosity_signal],proj)))
    entries=[]; seen=set()
    for entry in base["entries"]:
        key=(entry["factor"],entry["value"]); seen.add(key); mean=np.mean(samples[key],axis=0) if samples.get(key) else np.zeros(5); values=[int(np.clip(round(x),-1000,1000)) for x in mean]; entries.append({"factor":entry["factor"],"value":entry["value"],"delta_millis":values})
    for key, observed in sorted(samples.items()):
        if key in seen: continue
        mean=np.mean(observed,axis=0); entries.append({"factor":key[0],"value":key[1],"delta_millis":[int(np.clip(round(x),-1000,1000)) for x in mean]})
    result={"schema":"BCORE.LANGUAGE_STATE_CONTROLLER.V1","basis":["V","L1","L2","L3","L4"],"basis_scales":[1.0,1000.0,1000.0,1000.0,1000.0],"entries":entries,"train_observation_count":sum(len(v) for v in samples.values()),"validation_metrics":{"source":"APPROVED_STATE_CONTRAST_GOLD","semantic_inverse_failures":0.0},"blind_metrics":{"source":"SEALED_TEACHER_GOLD_HELDOUT","promotion_metric":"PENDING_RUNTIME_REEVALUATION"},"source_gold_sha256":gold["artifact_sha256"],"base_controller_sha256":base["artifact_sha256"],"promotion":"RESEARCH_SHADOW_ONLY","artifact_sha256":""};result["artifact_sha256"]=hashlib.sha256(json.dumps(result,ensure_ascii=False,separators=(",",":")).encode()).hexdigest();args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(result,ensure_ascii=False,indent=2),encoding="utf8");print(json.dumps({"output":str(args.output),"observations":result["train_observation_count"],"artifact_sha256":result["artifact_sha256"]},ensure_ascii=False,indent=2))

if __name__=="__main__":main()

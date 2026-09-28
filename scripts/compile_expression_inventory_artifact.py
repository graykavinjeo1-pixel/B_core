"""Compile generic expression transformations from approved feature evidence."""
from __future__ import annotations
import argparse, hashlib, json
from pathlib import Path

def main():
    ap=argparse.ArgumentParser();ap.add_argument("--gold",type=Path,required=True);ap.add_argument("--inventory",type=Path,required=True);ap.add_argument("--output",type=Path,required=True);args=ap.parse_args();gold=json.loads(args.gold.read_text(encoding='utf8')); inv=json.loads(args.inventory.read_text(encoding='utf8'))
    explicit=0; changed=0
    for pair in gold['pairs']:
        if pair['surface_pair_changed']:
            changed+=1
            if ('상태' in pair['left_surface']) != ('상태' in pair['right_surface']): explicit+=1
    rules=[]
    if explicit:
        rules.append({'transformation_id':'STATUS_RELATION_LABEL_EXPLICITNESS_V1','operation':'ENSURE_EXPLICIT_STATUS_LABEL','control_axis':'verbosity_millis','min_control_millis':100,'meaning_preserving':True,'source_transition_count':explicit,'surface_cache':False})
    result={'schema':'BCORE.EXPRESSION_CONSTRUCTION_INVENTORY.V1','source_gold_sha256':gold['artifact_sha256'],'source_feature_inventory_sha256':inv['artifact_sha256'],'surface_authority':'STRUCTURAL_TRANSFORMATIONS_ONLY_NO_TEACHER_SURFACE_CACHE','rules':rules,'runtime_promotion':'RESEARCH_SHADOW_ONLY','artifact_sha256':''};result['artifact_sha256']=hashlib.sha256(json.dumps(result,ensure_ascii=False,separators=(',',':')).encode()).hexdigest();args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(result,ensure_ascii=False,indent=2),encoding='utf8');print(json.dumps({'output':str(args.output),'rules':len(rules),'explicit_transitions':explicit,'changed_pairs':changed,'artifact_sha256':result['artifact_sha256']},ensure_ascii=False,indent=2))

if __name__=='__main__':main()

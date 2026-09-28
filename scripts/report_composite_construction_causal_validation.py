"""Seal a fail-closed causal-validation report for discovered composites."""
from __future__ import annotations
import argparse, hashlib, json
from pathlib import Path
from collections import Counter

def main():
 ap=argparse.ArgumentParser();ap.add_argument('--discovery',type=Path,required=True);ap.add_argument('--outputs',type=Path,required=True);ap.add_argument('--output',type=Path,required=True);a=ap.parse_args()
 d=json.loads(a.discovery.read_text(encoding='utf-8'));o=json.loads(a.outputs.read_text(encoding='utf-8'))
 candidates=[x for x in d['clusters'] if x['classification']=='COMPOSITE_PRIMITIVE']
 by={}
 for x in o:by.setdefault(x['id'],[]).append(x)
 transfer=[]
 for k,v in by.items():
  transfer.append({'sealed_blind_id':k,'variants':[{z['variant']:{'canonical_inverse':z['canonical_inverse'],'surface_sha256':hashlib.sha256(z['surface'].encode()).hexdigest()}} for z in v],'surface_changed':len({z['surface'] for z in v})>1})
 report={'schema':'BCORE.COMPOSITE_CONSTRUCTION_CAUSAL_VALIDATION.V1','runtime_installation':'FORBIDDEN','candidate_count':len(candidates),'candidates':[{'signature':x['signature'],'discovery_family_count':x['family_count'],'discovery_split_family_counts':x['split_family_counts'],'structural_components':x['signature'],'classification':'UNSTABLE','reason':'SEALED_BLIND_TRANSFER_TOO_SMALL_AND_COMPONENT_ABLATION_NOT_SEPARABLY_EXECUTABLE'} for x in candidates],'sealed_blind_register_proxy':{'tested_meaning_count':len(by),'outputs':len(o),'canonical_inverse_pass_count':sum(x['canonical_inverse'] for x in o),'surface_changed_count':sum(x['surface_changed'] for x in transfer),'speech_acts':'RECORDED_ONLY_NO_GLOBAL_TRANSFER_CLAIM','rows':transfer},'ablation':{'full_composite':'NOT_EXECUTABLE_AS_A_DISCOVERED_RUNTIME_UNIT','without_ending':'NOT_SEPARABLY_EXECUTABLE','without_lexical':'NOT_SEPARABLY_EXECUTABLE','without_honorific':'NOT_SEPARABLY_EXECUTABLE','without_length_change':'NOT_SEPARABLY_EXECUTABLE','single_component':'NOT_SEPARABLY_EXECUTABLE'},'conclusion':'NO_COMPOSITE_CONSTRUCTION_PROMOTION; EXISTING_REGISTER_REALIZER_ONLY_IS_NOT_EVIDENCE_FOR_DISCOVERED_COMPOSITE','artifact_sha256':''}
 report['artifact_sha256']=hashlib.sha256(json.dumps(report,ensure_ascii=False,sort_keys=True,separators=(',',':')).encode()).hexdigest();a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf-8');print(json.dumps({'candidates':len(candidates),'blind_inverse':report['sealed_blind_register_proxy']['canonical_inverse_pass_count'],'classification':'UNSTABLE'},ensure_ascii=False))
if __name__=='__main__':main()

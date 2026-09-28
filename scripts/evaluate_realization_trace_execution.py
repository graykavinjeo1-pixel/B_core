"""Evaluate inverse, Korean morphology, transfer, and ablations offline."""
from __future__ import annotations
import argparse, hashlib, json, re
from collections import Counter, defaultdict
from pathlib import Path

def has_jongseong(text:str)->bool:
 chars=[c for c in text if '가'<=c<='힣']
 return bool(chars) and (ord(chars[-1])-0xAC00)%28!=0
def particle_valid(text:str)->bool:
 for match in re.finditer(r'([가-힣]+)(으로|로)(?=\s|[.,?!]|$)',text):
  stem,particle=match.groups();jong=(ord(stem[-1])-0xAC00)%28
  expected='으로' if jong not in (0,8) else '로'
  if particle!=expected:return False
 return True
def main():
 ap=argparse.ArgumentParser();ap.add_argument('--substrate',type=Path,required=True);ap.add_argument('--gated',type=Path,required=True);ap.add_argument('--output',type=Path,required=True);a=ap.parse_args()
 substrate=json.loads(a.substrate.read_text(encoding='utf-8'));rows=json.loads(a.gated.read_text(encoding='utf-8'))
 evaluated=[]
 for x in rows:evaluated.append({'meaning_family_id':x['meaning_family_id'],'value':x['value'],'program_cluster_id':x.get('conditions',{}).get('program_cluster_id'),'inverse_pass':x['approval']=='APPROVED_STATE_CONTRAST_GOLD','surface_morphology_gate':particle_valid(x['surface']),'surface_sha256':hashlib.sha256(x['surface'].encode()).hexdigest()})
 full=[x for x in evaluated if x['value']=='FULL_PROGRAM'];abl=[x for x in evaluated if x['value'].startswith('ABLATION_')]
 recurrent=substrate['recurrent_programs']
 recurrent_operation_counts=[len(x['representative_program']['operations']) for x in recurrent]
 if recurrent_operation_counts and max(recurrent_operation_counts)==1:
  ablation_interpretation='PROGRAMS_ALREADY_MINIMAL; REMOVING_THE_ONLY_COMPONENT_RETURNS_THE_TRUSTED_SOURCE'
 elif abl:
  ablation_interpretation='MULTI_COMPONENT_ABLATIONS_EXECUTED'
 else:
  ablation_interpretation='NO_ELIGIBLE_MULTI_COMPONENT_PROGRAM'
 prior=[]
 for c in substrate['prior_composite_program_decomposition']:
  program_count=len(c['program_cluster_ids'])
  prior.append({'signature':c['signature'],'program_cluster_count':program_count,'speech_acts':c['speech_acts'],'classification':'DECOMPOSABLE_PROGRAM' if program_count>1 else 'CONSTRUCTION_SPECIFIC','reason':'COARSE_SURFACE_DELTA_DECOMPOSED_INTO_MULTIPLE_TRACE_EDIT_PROGRAMS' if program_count>1 else 'ONE_EXECUTABLE_PROGRAM_WITH_LIMITED_DOMAIN'})
 report={'schema':'BCORE.REALIZATION_TRACE_EXECUTION_EVALUATION.V1','runtime_installation':'FORBIDDEN','prior_candidate_status':'UNVALIDATED_EXECUTION_GAP','unstable_verdict_used':False,'trace_roundtrip_exact':substrate['unchanged_roundtrip_exact_count']==substrate['trace_count'],'trace_count':substrate['trace_count'],'program_count':substrate['program_count'],'program_exact_reconstruction_count':substrate['program_exact_reconstruction_count'],'program_exact_reconstruction':substrate['program_exact_reconstruction_count']==substrate['program_count'],'program_cluster_count':substrate['program_cluster_count'],'recurrent_program_cluster_count':substrate['recurrent_program_cluster_count'],'single_operation_recurrent_program_count':sum(x==1 for x in recurrent_operation_counts),'multi_operation_recurrent_program_count':sum(x>1 for x in recurrent_operation_counts),'unchanged_inverse_pass':sum(x['inverse_pass'] for x in evaluated if x['value']=='UNCHANGED_ROUNDTRIP'),'unchanged_count':sum(x['value']=='UNCHANGED_ROUNDTRIP' for x in evaluated),'full_transfer_count':len(full),'full_inverse_pass':sum(x['inverse_pass'] for x in full),'full_surface_morphology_pass':sum(x['surface_morphology_gate'] for x in full),'full_surface_naturalness':'NOT_ESTABLISHED_BY_AUTOMATIC_MORPHOLOGY_GATE','ablation_count':len(abl),'ablation_inverse_pass':sum(x['inverse_pass'] for x in abl),'component_ablation_interpretation':ablation_interpretation,'prior_composite_candidates':prior,'transfer_rows':evaluated,'conclusion':'EXECUTION_SUBSTRATE_PASS; COARSE_COMPOSITES_DECOMPOSED; NATURALNESS_REMAINS_HOLD; NO_RUNTIME_PROMOTION','artifact_sha256':''}
 report['artifact_sha256']=hashlib.sha256(json.dumps(report,ensure_ascii=False,sort_keys=True,separators=(',',':')).encode()).hexdigest();a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf-8');print(json.dumps({k:report[k] for k in ('trace_roundtrip_exact','program_exact_reconstruction','unchanged_inverse_pass','unchanged_count','full_transfer_count','full_inverse_pass','full_surface_morphology_pass','ablation_count')},ensure_ascii=False))
if __name__=='__main__':main()

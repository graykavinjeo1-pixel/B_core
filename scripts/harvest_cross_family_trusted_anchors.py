"""Harvest re-gated, provenance-linked anchors without creating new surfaces."""
from __future__ import annotations
import argparse,hashlib,json
from collections import Counter,defaultdict
from pathlib import Path
from analyze_paired_teacher_state_sensitivity import features
SOURCES=(
 ('existing_neutral','D:/B_Core_validation/expression_primitive_learning_v1/expression_primitive_candidates_ko_v1.json','D:/B_Core_validation/expression_primitive_learning_v1/expression_primitive_gated_ko_v1.json'),
 ('runtime_composition','D:/B_Core_validation/expression_rewrite_calibration_v1/recovered_runtime_anchor_candidates_ko_v1.json','D:/B_Core_validation/expression_rewrite_calibration_v1/recovered_runtime_anchor_gated_ko_v1.json'),
 ('independent_state','D:/B_Core_validation/natural_state_language_grounding_v1/natural_state_language_grounding_candidates_ko_v2.json','D:/B_Core_validation/natural_state_language_grounding_v1/natural_state_language_grounding_gated_ko_v2.json'),
 ('paired_state','D:/B_Core_validation/multi_factor_language_state_v1/paired_teacher_state_sensitivity_candidates_ko_v1.json','D:/B_Core_validation/multi_factor_language_state_v1/paired_teacher_state_sensitivity_gated_ko_v1.json'),
 ('speech_act_coverage','D:/B_Core_validation/multi_factor_language_state_v1/speech_act_state_relevance_candidates_ko_v1.json','D:/B_Core_validation/multi_factor_language_state_v1/speech_act_state_relevance_gated_ko_v1.json'),
)
def main():
 ap=argparse.ArgumentParser();ap.add_argument('--output',type=Path,required=True);args=ap.parse_args();anchors=[]
 for name,cp,gp in SOURCES:
  cpath,gpath=Path(cp),Path(gp)
  if not cpath.exists() or not gpath.exists():continue
  c=json.loads(cpath.read_text(encoding='utf8'));rows=c.get('rows',[]);g=json.loads(gpath.read_text(encoding='utf8'));ok={x['prompt_sha256']:x for x in g if x['approval']=='APPROVED_STATE_CONTRAST_GOLD'}
  for x in rows:
   z=ok.get(x.get('prompt_sha256'))
   if not z:continue
   if x.get('factor')=='EXPRESSION_PRIMITIVE' and x.get('value')!='NEUTRAL':continue
   f=features(x['surface']);anchors.append({'meaning_family_id':x['meaning_family_id'],'speech_act':x.get('speech_act',z.get('speech_act')),'source_artifact':name,'surface_sha256':hashlib.sha256(x['surface'].encode()).hexdigest(),'canonical_hash':hashlib.sha256(json.dumps(x['approved_response_ir'],ensure_ascii=False,sort_keys=True,separators=(',',':')).encode()).hexdigest() if 'approved_response_ir'in x else None,'inverse_exact':True,'unsupported_fact_count':0,'structure':{'clause_count':f['clause_count'],'explicit_subject':not f['subject_omitted'],'discourse_marker_present':f['discourse_marker']!='NONE','ending':f['ending_family'],'connective_present':any(k in x['surface'] for k in ('그리고','하지만','그래서','다만')),'modifier_rich':len(x['surface'])>=20,'lexical_variation_candidate':True}})
 # family diversity counts one family once per category
 fam={a['meaning_family_id'] for a in anchors}; cat=defaultdict(set)
 for a in anchors:
  s=a['structure'];f=a['meaning_family_id'];cat['one_clause' if s['clause_count']==1 else 'two_clause' if s['clause_count']==2 else 'three_plus_clause'].add(f);cat['explicit_subject' if s['explicit_subject'] else 'omitted_subject'].add(f);cat['discourse_present' if s['discourse_marker_present'] else 'discourse_absent'].add(f);cat['ending_'+s['ending'].lower()].add(f);cat['connective_present' if s['connective_present'] else 'connective_absent'].add(f);cat['modifier_rich' if s['modifier_rich'] else 'modifier_light'].add(f)
 matrix={k:{'family_count':len(v),'families':sorted(v)} for k,v in cat.items()};ready=len(fam)>=20 and all(len(cat[k])>=2 for k in ('one_clause','two_clause','three_plus_clause','explicit_subject','omitted_subject','discourse_present','discourse_absent','ending_formal','ending_polite'))
 out={'schema':'BCORE.CROSS_FAMILY_TRUSTED_ANCHOR_BANK.V1','runtime_installation':'FORBIDDEN','anchor_count':len(anchors),'unrelated_meaning_family_count':len(fam),'source_counts':dict(Counter(a['source_artifact'] for a in anchors)),'speech_act_counts':dict(Counter(a['speech_act'] for a in anchors)),'coverage_matrix':matrix,'readiness':'STRUCTURAL_ANCHOR_BANK_READY' if ready else 'INSUFFICIENT_STRUCTURAL_DIVERSITY','anchors':anchors,'artifact_sha256':''};out['artifact_sha256']=hashlib.sha256(json.dumps(out,ensure_ascii=False,separators=(',',':')).encode()).hexdigest();args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(out,ensure_ascii=False,indent=2),encoding='utf8');print(json.dumps({'anchors':len(anchors),'families':len(fam),'readiness':out['readiness']},ensure_ascii=False))
if __name__=='__main__':main()

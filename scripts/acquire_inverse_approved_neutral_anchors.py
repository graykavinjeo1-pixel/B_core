"""Build a provenance-preserving neutral anchor bank from already gated evidence."""
from __future__ import annotations
import argparse,hashlib,json
from collections import Counter
from pathlib import Path
from analyze_paired_teacher_state_sensitivity import features
def shape(s):
 f=features(s);return {'clause_count':f['clause_count'],'explicit_subject':not f['subject_omitted'],'discourse_marker':f['discourse_marker']!='NONE','ending':f['ending_family'],'modifier_like':any(x in s for x in ('오후','오전','현재','정기','예정','완료')),'lexical_alternative_candidate':True}
def main():
 ap=argparse.ArgumentParser();ap.add_argument('--candidates',type=Path,required=True);ap.add_argument('--gated',type=Path,required=True);ap.add_argument('--failed-candidates',type=Path,required=True);ap.add_argument('--failed-gated',type=Path,required=True);ap.add_argument('--output',type=Path,required=True);args=ap.parse_args();c=json.loads(args.candidates.read_text(encoding='utf8'))['rows'];g=json.loads(args.gated.read_text(encoding='utf8'));good={x['prompt_sha256']:x for x in g if x['approval']=='APPROVED_STATE_CONTRAST_GOLD'};bank=[]
 for x in c:
  if x['value']=='NEUTRAL' and x['prompt_sha256'] in good:
   z=good[x['prompt_sha256']];bank.append({'meaning_family_id':x['meaning_family_id'],'split':x['split'],'speech_act':x.get('speech_act'),'surface':x['surface'],'surface_sha256':hashlib.sha256(x['surface'].encode()).hexdigest(),'canonical_ir_provenance_sha256':hashlib.sha256(json.dumps(x['approved_response_ir'],ensure_ascii=False,sort_keys=True,separators=(',',':')).encode()).hexdigest(),'teacher_model':x['teacher_model'],'prompt_sha256':x['prompt_sha256'],'semantic_inverse':{'canonical_inverse':z['canonical_inverse'],'claim_same':z['claim_same'],'polarity_same':z['polarity_same'],'modality_same':z['modality_same'],'event_phase_same':z['event_phase_same'],'unsupported_fact_count':z['unsupported_fact_count']},'structure':shape(x['surface'])})
 fc=json.loads(args.failed_candidates.read_text(encoding='utf8'))['rows'];fg=json.loads(args.failed_gated.read_text(encoding='utf8'));reason=Counter()
 for x,z in zip(fc,fg):
  if z['approval']!='APPROVED_STATE_CONTRAST_GOLD':
   # Gate is fail-closed and does not expose a parse subtype; retain this generic class.
   reason['CANONICAL_INVERSE_NO_EXACT_MATCH_UNCLASSIFIED']+=1
 coverage={'short_1_clause':sum(x['structure']['clause_count']==1 for x in bank),'modifier_1_clause':sum(x['structure']['clause_count']==1 and x['structure']['modifier_like'] for x in bank),'explicit_subject':sum(x['structure']['explicit_subject'] for x in bank),'subject_omission':sum(not x['structure']['explicit_subject'] for x in bank),'two_clause':sum(x['structure']['clause_count']==2 for x in bank),'three_or_more_clause':sum(x['structure']['clause_count']>=3 for x in bank),'discourse_marker':sum(x['structure']['discourse_marker'] for x in bank),'ending_distribution':dict(Counter(x['structure']['ending'] for x in bank))}
 out={'schema':'BCORE.INVERSE_APPROVED_NEUTRAL_ANCHOR_BANK.V1','runtime_installation':'FORBIDDEN','primitive_learning':'FORBIDDEN','anchor_count':len(bank),'anchors':bank,'coverage':coverage,'new_neutral_reject_analysis':{'candidate_count':len(fc),'rejected_count':sum(reason.values()),'generic_reason_counts':dict(reason),'policy':'NO_PHRASE_EXCEPTION_NO_INVERSE_RELAXATION'},'readiness':'INSUFFICIENT_STRUCTURAL_DIVERSITY' if not coverage['two_clause'] or not coverage['three_or_more_clause'] else 'READY_FOR_REWRITE_CALIBRATION','artifact_sha256':''};out['artifact_sha256']=hashlib.sha256(json.dumps(out,ensure_ascii=False,separators=(',',':')).encode()).hexdigest();args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(out,ensure_ascii=False,indent=2),encoding='utf8');print(json.dumps({'output':str(args.output),'anchors':len(bank),'coverage':coverage,'readiness':out['readiness']},ensure_ascii=False))
if __name__=='__main__':main()

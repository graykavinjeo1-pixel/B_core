"""Discover structural expression transformations from inverse-approved pairs only."""
from __future__ import annotations
import argparse, hashlib, json, re
from collections import Counter, defaultdict
from pathlib import Path
from analyze_paired_teacher_state_sensitivity import features

CONNECTIVES=("그리고","하지만","그래서","다만","또는","면서","고 ")
def abstract_delta(before,after):
 a,b=features(before),features(after); d=[]
 for k in ('ending_family','honorific','deference','hedge','discourse_marker','emotion_marker','subject_omitted','information_order'):
  if a[k]!=b[k]: d.append(f'{k}:{a[k]}->{b[k]}')
 for k in ('clause_count','token_count','char_count'):
  if a[k]!=b[k]: d.append(f'{k}:{"UP" if b[k]>a[k] else "DOWN"}')
 at=set(re.findall(r'[가-힣A-Za-z0-9]+',before));bt=set(re.findall(r'[가-힣A-Za-z0-9]+',after))
 if at!=bt:d.append('lexical_substitution')
 ac=any(x in before for x in CONNECTIVES);bc=any(x in after for x in CONNECTIVES)
 if ac!=bc:d.append(f'connective:{"ADD" if bc else "REMOVE"}')
 if len(re.findall(r'\b(\w+)\s+\1\b',after))>len(re.findall(r'\b(\w+)\s+\1\b',before)):d.append('redundancy:UP')
 return tuple(sorted(d)) or ('IDENTICAL',)
def main():
 ap=argparse.ArgumentParser();ap.add_argument('--gated',type=Path,required=True);ap.add_argument('--output',type=Path,required=True);args=ap.parse_args(); rows=json.loads(args.gated.read_text(encoding='utf8'));ok=[r for r in rows if r['approval']=='APPROVED_STATE_CONTRAST_GOLD']
 ix={(r['meaning_family_id'],r['split'],r['value']):r for r in ok}; pairs=[]
 for (fam,split,op),after in ix.items():
  if op=='NEUTRAL':continue
  before=ix.get((fam,split,'NEUTRAL'))
  if before:pairs.append({'meaning_family_id':fam,'split':split,'instruction':op,'delta':abstract_delta(before['surface'],after['surface']),'changed':before['surface']!=after['surface']})
 by=defaultdict(list)
 for p in pairs:by[p['instruction']].append(p)
 summaries={}; sig_to_ops=defaultdict(set)
 for op,items in by.items():
  c=Counter(p['delta'] for p in items); dom,n=c.most_common(1)[0] if c else (('IDENTICAL',),0); train={p['delta'] for p in items if p['split']=='TRAIN' and p['delta']!=('IDENTICAL',)}; blind=[p for p in items if p['split']=='BLIND'];
  for s in c:sig_to_ops[s].add(op)
  summaries[op]={'pair_count':len(items),'changed_count':sum(p['changed'] for p in items),'signature_counts':{'|'.join(k):v for k,v in c.items()},'dominant_signature':'|'.join(dom),'dominant_rate':n/len(items) if items else 0,'blind_recurrent_count':sum(p['delta'] in train for p in blind),'blind_count':len(blind),'classification':'UNSTABLE_OR_INSUFFICIENT_PENDING_UNSEEN_AND_LONG_TEST'}
 aliases={'|'.join(sig):sorted(ops) for sig,ops in sig_to_ops.items() if sig!=('IDENTICAL',) and len(ops)>1}
 rejected=Counter(r['value'] for r in rows if r['approval']!='APPROVED_STATE_CONTRAST_GOLD')
 report={'schema':'BCORE.EXPRESSION_PRIMITIVE_STRUCTURAL_DISCOVERY.V1','source_gated_sha256':hashlib.sha256(args.gated.read_bytes()).hexdigest(),'surface_authority':'STRUCTURAL_DELTA_ONLY_NO_RAW_SURFACE_CACHE','runtime_promotion':'FORBIDDEN','approved_surface_count':len(ok),'rejected_by_instruction':dict(rejected),'neutral_operation':'BASELINE_ONLY','approved_neutral_operation_pairs':len(pairs),'instruction_summary':summaries,'cross_instruction_alias_candidates':aliases,'pair_deltas':pairs,'unseen_application':'NOT_STARTED_NO_RUNTIME_PRIMITIVE_PROMOTION','long_composition':'NOT_STARTED_NO_RUNTIME_PRIMITIVE_PROMOTION','artifact_sha256':''};report['artifact_sha256']=hashlib.sha256(json.dumps(report,ensure_ascii=False,separators=(',',':')).encode()).hexdigest();args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf8');print(json.dumps({'output':str(args.output),'pairs':len(pairs),'aliases':len(aliases),'artifact_sha256':report['artifact_sha256']},ensure_ascii=False))
if __name__=='__main__':main()

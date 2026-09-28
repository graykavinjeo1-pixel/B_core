"""Recover provenance-linked surfaces from existing runtime canary evidence."""
from __future__ import annotations
import argparse,hashlib,json
from pathlib import Path
def walk(x):
 if isinstance(x,dict):
  if isinstance(x.get('markdown'),str) and x.get('semantic_inverse') is True:yield x
  for v in x.values():yield from walk(v)
 elif isinstance(x,list):
  for v in x:yield from walk(v)
def main():
 ap=argparse.ArgumentParser();ap.add_argument('--campaign',type=Path,required=True);ap.add_argument('--canary',type=Path,required=True);ap.add_argument('--output',type=Path,required=True);a=ap.parse_args();camp=json.loads(a.campaign.read_text(encoding='utf8'));ir={r['approved_response']['semantic_sha256']:r['approved_response'] for r in camp['records']};data=json.loads(a.canary.read_text(encoding='utf8'));out=[]
 for o in walk(data):
  # Current canary is anchored to one canonical response; use only documented hash.
  h=data.get('source_response_sha256')
  if h in ir:out.append({'meaning_family_id':h,'split':data.get('source_split','BLIND'),'factor':'TRUSTED_EXISTING_RUNTIME_EVIDENCE','value':'RECOVERED','baseline_value':'RECOVERED','approved_response_ir':ir[h],'surface':o['markdown'],'teacher_model':'BCORE_RUNTIME_EVIDENCE','prompt_sha256':hashlib.sha256(o['markdown'].encode()).hexdigest()})
 a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps({'schema':'BCORE.STRUCTURAL_ANCHOR_CANDIDATES.V1','rows':out},ensure_ascii=False,indent=2),encoding='utf8');print(len(out))
if __name__=='__main__':main()

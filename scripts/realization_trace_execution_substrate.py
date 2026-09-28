"""Offline RealizationTrace, edit-program compiler, and generic executor.

This research substrate contains no style/persona labels and cannot install
programs into the production runtime.  Programs are compiled only from
inverse-approved semantic-equivalence pairs.
"""
from __future__ import annotations

import argparse, difflib, hashlib, itertools, json, re
from collections import Counter, defaultdict
from pathlib import Path

from build_semantic_equivalence_classes import slots

ENDINGS = tuple(sorted((
    "하셨습니까", "해 주십시오", "해 주세요", "하겠습니다", "할게요", "입니까", "인가요",
    "입니다", "이에요", "예요", "습니다", "습니까", "십시오", "세요", "어요", "아요", "니다", "했다", "해요", "해",
), key=len, reverse=True))
PARTICLES = tuple(sorted(("으로", "에서", "에게", "께서", "은", "는", "이", "가", "을", "를", "에", "로"), key=len, reverse=True))
DISCOURSE = ("우선", "먼저", "다만", "참고로", "그래도", "정리하면", "그러니까", "따라서", "하지만")
CONNECTIVES = ("그리고", "그러나", "그런데", "그래서", "따라서", "하지만", "다만", "또는")
TOKEN_RE = re.compile(r"<[^>]+>|[가-힣A-Za-z0-9]+|\s+|[^\w\s]", re.UNICODE)

def sha(value: object) -> str:
    return hashlib.sha256(json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()).hexdigest()

def normalize_with_bindings(surface: str, ir: dict) -> tuple[str, dict[str,str], list[str]]:
    normalized=surface; bindings={}; canonical_slots=[]; by_text=defaultdict(list)
    for text, token in slots(ir):
        canonical_slots.append(token)
        if token not in by_text[text]:by_text[text].append(token)
    for text,tokens in sorted(by_text.items(),key=lambda item:len(item[0]),reverse=True):
        for token in tokens:
            if text not in normalized:break
            normalized=normalized.replace(text,token,1);bindings[token]=text
    return normalized, bindings, sorted(set(canonical_slots))

def split_word(text: str) -> list[dict]:
    if text in PARTICLES:
        return [{"kind":"PARTICLE","text":text}]
    for ending in ENDINGS:
        if text.endswith(ending):
            stem=text[:-len(ending)]; result=[]
            if stem: result.append({"kind":"PREDICATE_LEXEME","text":stem,"morph":{"honorific":"시" in stem}})
            result.append({"kind":"ENDING_MORPHOLOGY","text":ending})
            return result
    for particle in PARTICLES:
        if text.endswith(particle) and len(text)>len(particle):
            return [{"kind":"LEXEME","text":text[:-len(particle)]},{"kind":"PARTICLE","text":particle}]
    kind="DISCOURSE_MARKER" if text in DISCOURSE else "CONNECTIVE" if text in CONNECTIVES else "LEXEME"
    return [{"kind":kind,"text":text}]

def refresh_trace_metadata(value: dict) -> dict:
    nodes=value["nodes"]
    clauses=[];start=0
    for i,node in enumerate(nodes):
        if node["kind"]=="PUNCTUATION" and node["text"] in ".?!": clauses.append([start,i+1]);start=i+1
    if start<len(nodes):clauses.append([start,len(nodes)])
    slot_realizations=[]
    for index,node in enumerate(nodes):
        if node["kind"]!="SEMANTIC_SLOT":continue
        following=next((candidate for candidate in nodes[index+1:] if candidate["kind"]!="SPACE"),None)
        particle=following["text"] if following and following["kind"]=="PARTICLE" else None
        default_role="SUBJECT" if node["text"].startswith("<SUBJECT_") else "VALUE"
        role=("TOPIC" if particle in {"은","는"} else "SUBJECT" if particle in {"이","가","께서"}
              else "OBJECT" if particle in {"을","를"} else default_role)
        slot_realizations.append({"slot":node["text"],"semantic_role":role,"node_index":index,"surface_binding":node.get("binding"),"nearby_particle":particle})
    realized={item["slot"] for item in slot_realizations}
    value.update({
        "clause_structure": clauses,
        "clause_order": list(range(len(clauses))),
        "slot_realizations": slot_realizations,
        "omitted_slots": sorted(set(value.get("canonical_slot_ids", [])) - realized),
        "predicate_lexical_realization": [n["text"] for n in nodes if n["kind"] == "PREDICATE_LEXEME"],
        "ending_morphology": [n["text"] for n in nodes if n["kind"] == "ENDING_MORPHOLOGY"],
        "honorific_morphology": any(n.get("morph", {}).get("honorific") for n in nodes),
        "discourse_markers": [n["text"] for n in nodes if n["kind"] == "DISCOURSE_MARKER"],
        "connectives": [n["text"] for n in nodes if n["kind"] == "CONNECTIVE"],
        "particles": [n["text"] for n in nodes if n["kind"] == "PARTICLE"],
        "lexical_variants": [n["text"] for n in nodes if n["kind"] in {"LEXEME", "PREDICATE_LEXEME"}],
    })
    return value

def trace(surface: str, ir: dict) -> dict:
    normalized,bindings,canonical_slots=normalize_with_bindings(surface,ir);nodes=[]
    for token in TOKEN_RE.findall(normalized):
        if token.isspace(): nodes.append({"kind":"SPACE","text":token})
        elif token.startswith("<"): nodes.append({"kind":"SEMANTIC_SLOT","text":token,"binding":bindings.get(token)})
        elif re.fullmatch(r"[가-힣A-Za-z0-9]+",token): nodes.extend(split_word(token))
        else: nodes.append({"kind":"PUNCTUATION","text":token})
    return refresh_trace_metadata({"schema":"BCORE.REALIZATION_TRACE.V1","canonical_hash":sha(ir),"surface_sha256":hashlib.sha256(surface.encode()).hexdigest(),"slot_bindings":bindings,"canonical_slot_ids":canonical_slots,"nodes":nodes})

def realize(t: dict) -> str:
    normalized="".join(node["text"] for node in t["nodes"])
    for token,text in sorted(t["slot_bindings"].items(),key=lambda x:len(x[0]),reverse=True): normalized=normalized.replace(token,text)
    return normalized

def ending(t: dict):
    return next((n["text"] for n in reversed(t["nodes"]) if n["kind"]=="ENDING_MORPHOLOGY"),None)

def compile_program(a: dict,b: dict) -> dict:
    ops=[]; ea,eb=ending(a),ending(b)
    if ea != eb and ea and eb: ops.append({"op":"CHANGE_ENDING","from":ea,"to":eb})
    sa=json.loads(json.dumps(a["nodes"],ensure_ascii=False));sb=b["nodes"]
    if ea != eb and ea and eb:
        hit=next((n for n in reversed(sa) if n["kind"]=="ENDING_MORPHOLOGY" and n["text"]==ea),None)
        if hit:hit["text"]=eb
    ka=[(n["kind"],n["text"]) for n in sa]
    kb=[(n["kind"],n["text"]) for n in sb]
    matcher=difflib.SequenceMatcher(a=ka,b=kb,autojunk=False)
    for tag,i1,i2,j1,j2 in matcher.get_opcodes():
        if tag=='equal':continue
        left=sa[i1:i2];right=sb[j1:j2]
        context={"left_context":sa[i1-1] if i1 else None,"right_context":sa[i2] if i2<len(sa) else None}
        if tag=='insert':ops.append({"op":"INSERT_NODE","nodes":right,**context})
        elif tag=='delete':ops.append({"op":"DELETE_OPTIONAL_NODE","nodes":left,**context})
        else:ops.append({"op":"SUBSTITUTE_VARIANT","from_nodes":left,"to_nodes":right})
    return {"schema":"BCORE.REALIZATION_EDIT_PROGRAM.V1","operations":ops}

def canonical_program(program: dict) -> str:
    canonical=[]
    for op in program["operations"]:
        kind=op["op"]
        if kind=='CHANGE_ENDING':canonical.append((kind,tuple(sorted((op['from'],op['to'])))))
        elif kind=='SUBSTITUTE_VARIANT':canonical.append((kind,tuple(n['kind'] for n in op['from_nodes']),tuple(n['kind'] for n in op['to_nodes'])))
        elif kind in {'INSERT_NODE','DELETE_OPTIONAL_NODE'}:canonical.append((kind,tuple(n['kind'] for n in op['nodes'])))
        else:canonical.append((kind,op.get('from'),op.get('to')))
    return sha(canonical)

def replace_sequence(nodes:list[dict],old:list[dict],new:list[dict]) -> bool:
    sig=lambda n:(n['kind'],n['text'])
    target=[sig(n) for n in old]
    for i in range(len(nodes)-len(old)+1):
        if [sig(n) for n in nodes[i:i+len(old)]]==target:nodes[i:i+len(old)]=[dict(n) for n in new];return True
    return False

def insert_at_context(nodes:list[dict],new:list[dict],left:dict|None,right:dict|None) -> bool:
    sig=lambda n:(n['kind'],n['text'])
    left_sig=sig(left) if left else None;right_sig=sig(right) if right else None
    for index in range(len(nodes)+1):
        left_ok=left_sig is None if index==0 else left_sig is not None and sig(nodes[index-1])==left_sig
        right_ok=right_sig is None if index==len(nodes) else right_sig is not None and sig(nodes[index])==right_sig
        if left_ok and right_ok:nodes[index:index]=[dict(n) for n in new];return True
    return False

def execute(source:dict,program:dict,reverse=False,drop_index=None) -> tuple[dict,bool]:
    result=json.loads(json.dumps(source,ensure_ascii=False)); applied=True
    operations=list(program['operations'])
    if reverse:operations=list(reversed(operations))
    for index,raw in enumerate(operations):
        if index==drop_index:continue
        op=dict(raw);kind=op['op']
        if reverse:
            if kind=='CHANGE_ENDING':op['from'],op['to']=op['to'],op['from']
            elif kind=='SUBSTITUTE_VARIANT':op['from_nodes'],op['to_nodes']=op['to_nodes'],op['from_nodes']
            elif kind=='INSERT_NODE':kind='DELETE_OPTIONAL_NODE';op={'op':kind,'nodes':op['nodes']}
            elif kind=='DELETE_OPTIONAL_NODE':kind='INSERT_NODE';op={'op':kind,'nodes':op['nodes'],'left_context':op.get('left_context'),'right_context':op.get('right_context')}
        if kind=='CHANGE_ENDING':
            hit=next((n for n in reversed(result['nodes']) if n['kind']=='ENDING_MORPHOLOGY' and n['text']==op['from']),None)
            if hit:hit['text']=op['to']
            else:applied=False
        elif kind=='SUBSTITUTE_VARIANT': applied=replace_sequence(result['nodes'],op['from_nodes'],op['to_nodes']) and applied
        elif kind=='DELETE_OPTIONAL_NODE': applied=replace_sequence(result['nodes'],op['nodes'],[]) and applied
        elif kind=='INSERT_NODE':
            applied=insert_at_context(result['nodes'],op['nodes'],op.get('left_context'),op.get('right_context')) and applied
        else: applied=False
    refresh_trace_metadata(result)
    result['surface_sha256']=hashlib.sha256(realize(result).encode()).hexdigest()
    return result,applied

def particle_morphology_valid(text:str)->bool:
    for match in re.finditer(r'([가-힣]+)(으로|로)(?=\s|[.,?!]|$)',text):
        stem,particle=match.groups();jong=(ord(stem[-1])-0xAC00)%28
        if particle != ('으로' if jong not in (0,8) else '로'):return False
    return True

def main():
    ap=argparse.ArgumentParser();ap.add_argument('--classes',type=Path,required=True);ap.add_argument('--discovery',type=Path,required=True);ap.add_argument('--trace-output',type=Path,required=True);ap.add_argument('--candidates-output',type=Path,required=True);args=ap.parse_args()
    data=json.loads(args.classes.read_text(encoding='utf-8')); discovery=json.loads(args.discovery.read_text(encoding='utf-8'))
    traces={}; programs=[];clusters=defaultdict(list)
    for c in data['classes']:
        for s in c['surfaces']:traces[s['surface_sha256']]=trace(s['surface'],c['approved_response_ir'])
        for a,b in itertools.combinations(c['surfaces'],2):
            program=compile_program(traces[a['surface_sha256']],traces[b['surface_sha256']]);key=canonical_program(program)
            row={'program_id':sha({'family':c['meaning_family_id'],'a':a['surface_sha256'],'b':b['surface_sha256']}),'program_cluster_id':key,'meaning_family_id':c['meaning_family_id'],'split':c['split'],'speech_act':c['speech_act'],'source_trace_sha256':a['surface_sha256'],'target_trace_sha256':b['surface_sha256'],'program':program}
            programs.append(row);clusters[key].append(row)
    recurrent=[]
    for key,rows in clusters.items():
        families={r['meaning_family_id'] for r in rows}; splits=Counter(r['split'] for r in rows)
        if len(families)>=3 and all(splits[s]>0 for s in ('TRAIN','VALIDATION','BLIND')):recurrent.append({'program_cluster_id':key,'family_count':len(families),'split_pair_counts':dict(splits),'representative_program':rows[0]['program']})
    # A class with only one surface contributed no pair and therefore no edit
    # program to discovery.  Seal every such family as the transfer holdout,
    # independent of its historical acquisition split.
    holdouts=[c for c in data['classes'] if c['surface_count']==1 and particle_morphology_valid(c['surfaces'][0]['surface'])]
    candidates=[]
    for c in holdouts:
        source=c['surfaces'][0];source_trace=traces[source['surface_sha256']]
        # unchanged round-trip control
        candidates.append({'meaning_family_id':c['meaning_family_id'],'split':'BLIND','factor':'REALIZATION_TRACE','value':'UNCHANGED_ROUNDTRIP','baseline_value':'SOURCE','approved_response_ir':c['approved_response_ir'],'surface':realize(source_trace),'teacher_model':'NONE_OFFLINE_GENERIC_EXECUTOR','prompt_sha256':sha({'control':source['surface_sha256']}),'conditions':{'source_surface_sha256':source['surface_sha256']}})
        for cluster in recurrent:
            for reverse in (False,True):
                transformed,ok=execute(source_trace,cluster['representative_program'],reverse=reverse)
                if ok and realize(transformed)!=source['surface']:
                    candidates.append({'meaning_family_id':c['meaning_family_id'],'split':'BLIND','factor':'REALIZATION_TRACE','value':'FULL_PROGRAM','baseline_value':'SOURCE','approved_response_ir':c['approved_response_ir'],'surface':realize(transformed),'teacher_model':'NONE_OFFLINE_GENERIC_EXECUTOR','prompt_sha256':sha({'source':source['surface_sha256'],'program':cluster['program_cluster_id'],'reverse':reverse}),'conditions':{'source_surface_sha256':source['surface_sha256'],'program_cluster_id':cluster['program_cluster_id'],'reverse':str(reverse)}})
                    for drop_index in range(len(cluster['representative_program']['operations'])):
                        ablated,ablation_ok=execute(source_trace,cluster['representative_program'],reverse=reverse,drop_index=drop_index)
                        if ablation_ok and realize(ablated)!=source['surface']:
                            candidates.append({'meaning_family_id':c['meaning_family_id'],'split':'BLIND','factor':'REALIZATION_TRACE','value':f'ABLATION_DROP_{drop_index}','baseline_value':'SOURCE','approved_response_ir':c['approved_response_ir'],'surface':realize(ablated),'teacher_model':'NONE_OFFLINE_GENERIC_EXECUTOR','prompt_sha256':sha({'source':source['surface_sha256'],'program':cluster['program_cluster_id'],'reverse':reverse,'drop':drop_index}),'conditions':{'source_surface_sha256':source['surface_sha256'],'program_cluster_id':cluster['program_cluster_id'],'reverse':str(reverse),'drop_index':str(drop_index)}})
    program_by_pair={frozenset((p['source_trace_sha256'],p['target_trace_sha256'])):p for p in programs}
    prior_decomposition=[]
    for candidate in [x for x in discovery['clusters'] if x['classification']=='COMPOSITE_PRIMITIVE']:
        matched=[]
        for pair in discovery['pairs']:
            if pair['signature']!=candidate['signature']:continue
            program=program_by_pair.get(frozenset((pair['left_surface_sha256'],pair['right_surface_sha256'])))
            if program:matched.append(program)
        prior_decomposition.append({'signature':candidate['signature'],'program_cluster_ids':sorted({p['program_cluster_id'] for p in matched}),'speech_acts':sorted({p['speech_act'] for p in matched}),'operation_sequences':[p['program']['operations'] for p in matched]})
    exact_programs=0
    for program in programs:
        transformed,ok=execute(traces[program['source_trace_sha256']],program['program'])
        exact_programs+=int(ok and realize(transformed)==realize(traces[program['target_trace_sha256']]))
    artifact={'schema':'BCORE.REALIZATION_TRACE_EXECUTION_SUBSTRATE.V1','runtime_installation':'FORBIDDEN','style_labels_present':False,'trace_count':len(traces),'unchanged_roundtrip_exact_count':sum(realize(t)==next(s['surface'] for c in data['classes'] for s in c['surfaces'] if s['surface_sha256']==k) for k,t in traces.items()),'program_count':len(programs),'program_exact_reconstruction_count':exact_programs,'program_cluster_count':len(clusters),'recurrent_program_cluster_count':len(recurrent),'edit_instruction_schema':['INSERT_NODE','DELETE_OPTIONAL_NODE','SUBSTITUTE_VARIANT','CHANGE_MORPH_FEATURE','CHANGE_ENDING','REORDER','SPLIT_CLAUSE','MERGE_CLAUSE'],'executor_supported_in_v1':['INSERT_NODE','DELETE_OPTIONAL_NODE','SUBSTITUTE_VARIANT','CHANGE_ENDING'],'executor_deferred_until_evidence':['CHANGE_MORPH_FEATURE','REORDER','SPLIT_CLAUSE','MERGE_CLAUSE'],'traces':list(traces.values()),'programs':programs,'recurrent_programs':recurrent,'prior_composite_candidates':[x for x in discovery['clusters'] if x['classification']=='COMPOSITE_PRIMITIVE'],'prior_composite_program_decomposition':prior_decomposition,'artifact_sha256':''}
    artifact['artifact_sha256']=sha(artifact);args.trace_output.parent.mkdir(parents=True,exist_ok=True);args.trace_output.write_text(json.dumps(artifact,ensure_ascii=False,indent=2),encoding='utf-8')
    args.candidates_output.write_text(json.dumps({'schema':'BCORE.REALIZATION_TRACE_TRANSFER_CANDIDATES.V1','runtime_installation':'FORBIDDEN','rows':candidates},ensure_ascii=False,indent=2),encoding='utf-8')
    print(json.dumps({'traces':len(traces),'programs':len(programs),'clusters':len(clusters),'recurrent':len(recurrent),'holdouts':len(holdouts),'candidates':len(candidates)},ensure_ascii=False))
if __name__=='__main__':main()

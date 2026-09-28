"""Localhost-only UI for an actual human naturalness calibration review."""
from __future__ import annotations

import argparse
import json
import threading
from datetime import datetime, timezone
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


HTML = r"""<!doctype html>
<html lang="ko"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>B_Core 자연스러움 사람 평가</title>
<style>
body{font-family:system-ui,'Malgun Gothic',sans-serif;background:#f4f5f7;color:#18202a;margin:0}.wrap{max-width:900px;margin:28px auto;padding:0 18px}.card{background:white;border:1px solid #d8dde5;border-radius:14px;padding:24px;box-shadow:0 3px 14px #0001}.surface{font-size:25px;line-height:1.55;padding:20px;background:#f8fafc;border-radius:10px;margin:16px 0}.meaning{font-size:14px;white-space:pre-wrap;background:#f1f4f8;padding:12px;border-radius:8px}.group{margin:18px 0}.group strong{display:block;margin-bottom:8px}.buttons{display:flex;gap:8px;flex-wrap:wrap}button,label.choice{border:1px solid #aeb7c4;background:#fff;border-radius:8px;padding:10px 14px;cursor:pointer}label.choice:has(input:checked){background:#dcecff;border-color:#3984da}input[type=radio]{margin-right:6px}.nav{display:flex;justify-content:space-between;margin-top:20px}.progress{height:8px;background:#dfe4ea;border-radius:8px;overflow:hidden}.progress>div{height:100%;background:#2678d4}.meta{display:flex;justify-content:space-between;margin:10px 0;color:#5a6675}.submit{background:#1769c2;color:white;border:0}.submit:disabled{background:#9ba8b5}.notice{color:#6c3d00;background:#fff4d7;padding:10px;border-radius:8px}.done{color:#126335;background:#e4f7eb;padding:18px;border-radius:10px;font-size:18px}</style></head>
<body><div class="wrap"><h1>B_Core 자연스러움 사람 평가</h1><p class="notice">문장 출처와 자동 판정은 가려져 있습니다. 의미 요약과 비교해 한국어 자체를 평가해 주세요. 이 평가는 verifier 보정 증거이며 runtime 문장 authority가 아닙니다.</p><div class="progress"><div id="bar"></div></div><div class="meta"><span id="count"></span><span id="saved"></span></div><div id="app" class="card"></div></div>
<script>
let packet,idx=0; const choices=(name,values,current)=>`<div class="buttons">${values.map(([v,t])=>`<label class="choice"><input type="radio" name="${name}" value="${v}" ${String(current)===v?'checked':''}>${t}</label>`).join('')}</div>`;
async function init(){packet=await (await fetch('/api/packet')).json();render()}
function itemComplete(x){return typeof x.natural_korean==='boolean'&&typeof x.meaning_preserved==='boolean'&&typeof x.contextually_usable==='boolean'&&['NATURAL','AWKWARD_BUT_GRAMMATICAL','UNNATURAL'].includes(x.acceptability)}
function capture(){const x=packet.items[idx];for(const n of ['natural_korean','meaning_preserved','contextually_usable']){const e=document.querySelector(`input[name=${n}]:checked`);if(e)x[n]=e.value==='true'}const a=document.querySelector('input[name=acceptability]:checked');if(a)x.acceptability=a.value;x.notes=document.querySelector('#notes')?.value||null}
async function save(){capture();await fetch('/api/save',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(packet)});document.querySelector('#saved').textContent='저장됨'}
async function move(d){await save();idx=Math.max(0,Math.min(packet.items.length-1,idx+d));render()}
function render(){const x=packet.items[idx],done=packet.items.filter(itemComplete).length;document.querySelector('#bar').style.width=`${done/packet.items.length*100}%`;document.querySelector('#count').textContent=`${idx+1}/${packet.items.length} · 완료 ${done}`;document.querySelector('#app').innerHTML=`<div class="surface">${x.surface}</div><strong>승인 의미</strong><div class="meaning">${JSON.stringify(x.canonical_summary,null,2)}</div><div class="group"><strong>자연스러운 한국어인가?</strong>${choices('natural_korean',[['true','예'],['false','아니오']],x.natural_korean)}</div><div class="group"><strong>승인 의미가 그대로 보존됐는가?</strong>${choices('meaning_preserved',[['true','예'],['false','아니오']],x.meaning_preserved)}</div><div class="group"><strong>실제 대화 맥락에서 사용할 수 있는가?</strong>${choices('contextually_usable',[['true','예'],['false','아니오']],x.contextually_usable)}</div><div class="group"><strong>종합 수용성</strong>${choices('acceptability',[['NATURAL','자연스러움'],['AWKWARD_BUT_GRAMMATICAL','문법적이나 어색함'],['UNNATURAL','부자연스러움']],x.acceptability)}</div><div class="group"><strong>선택 메모</strong><textarea id="notes" rows="3" style="width:100%">${x.notes||''}</textarea></div><div class="nav"><button onclick="move(-1)" ${idx===0?'disabled':''}>이전</button><button onclick="move(1)">${idx===packet.items.length-1?'저장/완료':'다음'}</button></div>${done===packet.items.length?`<hr><div class="group"><strong>평가자 식별자</strong><input id="reviewer" placeholder="이름 또는 별칭" style="padding:10px;width:280px"></div><label><input id="attest" type="checkbox"> 제가 직접 모든 항목을 평가했으며 자동 모델 판정을 사람 평가로 제출하지 않았습니다.</label><p><button class="submit" onclick="submitReview()">최종 제출</button></p>`:''}`}
async function submitReview(){capture();const reviewer=document.querySelector('#reviewer').value.trim(),attest=document.querySelector('#attest').checked;if(!reviewer||!attest){alert('평가자 식별자와 직접 평가 확인이 필요합니다.');return}packet.reviewer.reviewer_id=reviewer;packet.reviewer.attestation='ACTUAL_HUMAN_REVIEW_COMPLETED';packet.reviewer.reviewed_at=new Date().toISOString();const r=await fetch('/api/submit',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(packet)});const x=await r.json();if(!r.ok){alert(x.error);return}document.querySelector('#app').innerHTML=`<div class="done">평가가 저장되었습니다. 이 창을 닫아도 됩니다.<br><small>${x.output}</small></div>`}
document.addEventListener('change',()=>{capture();document.querySelector('#saved').textContent='저장 대기'});init();
</script></body></html>"""


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--packet", type=Path, required=True)
    ap.add_argument("--draft", type=Path, required=True)
    ap.add_argument("--completed", type=Path, required=True)
    ap.add_argument("--port", type=int, default=8765)
    args = ap.parse_args()
    original = json.loads(args.packet.read_text(encoding="utf-8"))
    state = json.loads(args.draft.read_text(encoding="utf-8")) if args.draft.exists() else original

    class Handler(BaseHTTPRequestHandler):
        def _json(self, value: object, status: int = 200):
            data = json.dumps(value, ensure_ascii=False).encode()
            self.send_response(status); self.send_header("Content-Type", "application/json; charset=utf-8")
            self.send_header("Content-Length", str(len(data))); self.end_headers(); self.wfile.write(data)
        def do_GET(self):
            if self.path == "/api/packet": return self._json(state)
            data = HTML.encode()
            self.send_response(200); self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(data))); self.end_headers(); self.wfile.write(data)
        def do_POST(self):
            length = int(self.headers.get("Content-Length", "0"))
            payload = json.loads(self.rfile.read(length))
            if payload.get("schema") != "BCORE.HUMAN_NATURALNESS_CALIBRATION_PACKET.V2":
                return self._json({"error": "SCHEMA_MISMATCH"}, 400)
            state.clear(); state.update(payload)
            args.draft.parent.mkdir(parents=True, exist_ok=True)
            args.draft.write_text(json.dumps(state, ensure_ascii=False, indent=2), encoding="utf-8")
            if self.path == "/api/submit":
                incomplete = [row["review_id"] for row in state["items"] if row.get("acceptability") not in {"NATURAL", "AWKWARD_BUT_GRAMMATICAL", "UNNATURAL"} or not all(isinstance(row.get(k), bool) for k in ("natural_korean", "meaning_preserved", "contextually_usable"))]
                if incomplete or state.get("reviewer", {}).get("attestation") != "ACTUAL_HUMAN_REVIEW_COMPLETED":
                    return self._json({"error": "INCOMPLETE_OR_UNATTESTED_REVIEW", "items": incomplete}, 400)
                args.completed.write_text(json.dumps(state, ensure_ascii=False, indent=2), encoding="utf-8")
                self._json({"saved": True, "output": str(args.completed)})
                threading.Timer(0.5, self.server.shutdown).start()
                return
            self._json({"saved": True})
        def log_message(self, format, *values):
            return

    server = ThreadingHTTPServer(("127.0.0.1", args.port), Handler)
    print(json.dumps({"url": f"http://127.0.0.1:{args.port}/", "items": len(state["items"]), "completed": str(args.completed)}, ensure_ascii=False), flush=True)
    server.serve_forever()


if __name__ == "__main__":
    main()

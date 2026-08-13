# B_Core Cognitive API

`b-core-cognitive-api` is a local JSON Lines API that keeps one bounded
experience-memory session alive on standard input/output. It does not use the
network or an external language model.

The executable accepts UTF-8, UTF-8 BOM, UTF-16LE, and UTF-16BE input streams,
including the native pipeline encoding used by Windows PowerShell.

Windows PowerShell 5.1 can replace non-ASCII pipeline text before the process
receives it. Set UTF-8 before sending Korean input:

```powershell
$OutputEncoding = [System.Text.UTF8Encoding]::new($false)
```

## Product path

```text
Korean or English text
→ typed LanguageUnderstandingIR
→ typed PlanGoalIR
→ core-owned dependency-ordered PlanIR
→ Korean or English output grounded in the PlanIR
```

The language adapter owns surface-form interpretation. The dockable semantic
core owns experience recall and plan structure. Natural-language text never
becomes executable semantic authority by itself.

## Commands

Each input line is one `CognitiveApiCommandIR` JSON object. Each output line is
one `CognitiveApiResponseIR`.

Supported operations:

- `INJECT_EXPERIENCE`
- `EXPORT_EXPERIENCE_SNAPSHOT`
- `IMPORT_EXPERIENCE_SNAPSHOT`
- `INJECT_LANGUAGE_KNOWLEDGE`
- `PROCESS_NATURAL_LANGUAGE`
- `LANGUAGE_KNOWLEDGE_STATISTICS`

Example natural-language request:

```json
{"operation":"PROCESS_NATURAL_LANGUAGE","request":{"schema":"B_CORE_NATURAL_LANGUAGE_REQUEST_1","request_id":"REQ-1","text":"경로 결함을 점검하고 수리 계획 세워줘. ㄱㄱ","output_language":"KOREAN","context_tags":["path"],"max_plan_steps":12}}
```

Example successful-experience injection:

```json
{"operation":"INJECT_EXPERIENCE","experience":{"schema":"B_CORE_EXPERIENCE_IR_1","experience_id":"EXP-PATH-1","situation":"PowerShell 경로 처리 실패","action":"정확한 LiteralPath를 사용","outcome":"SUCCESSFUL","outcome_description":"빌드와 경로 검증 통과","semantic_tags":["path","powershell","repair"],"evidence":["exit_code=0"],"confidence_millis":950,"source_language":"ko"}}
```

## Experience contract

Experience identity is content-addressed and bounded. Reusing an identity with
different content fails closed. Snapshot import validates every entry and the
aggregate hash before changing memory. Snapshot import/export is the explicit
persistence mechanism; episodic memory does not mutate the sealed semantic
state.

All outcomes may be recalled for diagnosis. Only `SUCCESSFUL` experiences are
attached to candidate-generation and consequence-prediction steps as reusable
solution evidence.

## Language knowledge

The built-in typed knowledge base contains representative Korean and English:

- grammar and discourse markers;
- task and intent words;
- idioms;
- informal slang;
- internet language and current colloquial expressions.

Additional knowledge can be injected as `LanguageKnowledgeEntryIR`. Entries
carry language, category, register, canonical concept, semantic tags, optional
intent hints, and pragmatic function. They are not stored as untyped prose.

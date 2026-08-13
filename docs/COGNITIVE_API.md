# B_Core Cognitive API

`b-core-cognitive-api` is a persistent local JSON Lines API. It combines
bounded experience recall, context-sensitive lexical memory, natural-language
planning, and typed knowledge-document work without a network or external LLM.

The executable accepts UTF-8, UTF-8 BOM, UTF-16LE, and UTF-16BE input streams.
Windows PowerShell 5.1 must be told to preserve non-ASCII pipeline text:

```powershell
$OutputEncoding = [System.Text.UTF8Encoding]::new($false)
```

## Runtime path

```text
Korean or English command
→ surface/discourse interpretation
→ LexemeIR surface matching
→ context-weighted SenseIR activation
→ core-owned dependency-ordered PlanIR
→ PaperIR | BusinessDocumentIR | TableIR | ChartIR | FinancialStatementIR | PlanProposalIR
→ evidence-bound findings
→ genre-aware DocumentDesignIR
→ Markdown | print-ready HTML | JSON | CSV | SVG
→ text, file, or both
```

Lexical frequency is only a prior. Context selectors, domain, collocation,
confidence, verified successful use, ambiguity, and rejected activations also
affect the score. A successful outcome credits the selected `SenseIR`, not every
meaning of the same spelling.

## Commands

Each input line is one `CognitiveApiCommandIR` object. Each output line is one
`CognitiveApiResponseIR` object.

- `INJECT_EXPERIENCE`
- `EXPORT_EXPERIENCE_SNAPSHOT`
- `IMPORT_EXPERIENCE_SNAPSHOT`
- `INJECT_LANGUAGE_KNOWLEDGE` (legacy discourse/surface knowledge)
- `INJECT_LEXEME`
- `EXPORT_LEXEME_SNAPSHOT`
- `IMPORT_LEXEME_SNAPSHOT`
- `RECORD_LEXICAL_OUTCOME`
- `PROCESS_NATURAL_LANGUAGE`
- `PROCESS_KNOWLEDGE_WORK`
- `LANGUAGE_KNOWLEDGE_STATISTICS`
- `LEXICAL_MEMORY_STATISTICS`

## Lexeme and sense memory

`LexemeIR` contains:

- language, lemma, inflected forms, part of speech, and grammatical roles;
- one or more `SenseIR` values;
- synonym, antonym, hypernym, hyponym, entailment, and related-sense edges;
- collocations, domains, source, confidence, and frequency prior.

Each sense contains its canonical concept, gloss, semantic tags, context
selectors, relations, optional plan-intent hint, and confidence. Dynamic
lexemes and their usage weights are preserved through explicit snapshot
export/import. Reusing an identity with different content fails closed.

Only a verified outcome with evidence changes success/rejection weights:

```json
{"operation":"RECORD_LEXICAL_OUTCOME","outcome":{"activation_keys":["KO.TABLE/KO.TABLE.S1"],"verified_success":true,"evidence":["human-confirmed table interpretation"]}}
```

## Knowledge work

The natural-language command determines `INTERPRET`, `ANALYZE`, `WRITE`,
`PLAN`, or `REVISE`. Activated lexical concepts can introduce new command words
without adding another hard-coded command branch. The optional explicit
`document_kind` is authoritative when the operator needs to disambiguate the
artifact. Optional `output_language` accepts `KOREAN` or `ENGLISH`; otherwise
the command language is used. Generated paper/plan structures, analysis
findings, and Markdown labels follow that resolved language.

Supported document IR:

- `PaperIR`: title, authors, abstract, hierarchical sections, claims,
  evidence locations, references, tables, and charts;
- `BusinessDocumentIR`: distinct business-plan and business-proposal contracts,
  executive summary, organization, audience, evidence-bound metrics, strategic
  sections, tables, charts, financial statements, execution roadmap, risks, and
  next action;
- `TableIR`: typed cells, exact decimal values, missing values, row/column
  structure, and provenance locations;
- `ChartIR`: chart type, axes, series, exact numeric points, and source
  locations;
- `FinancialStatementIR`: entity, statement type, periods, currency, unit,
  classified line items, and exact period values;
- `PlanProposalIR`: objective, dependency-bearing tasks, completion
  conditions, risks, and assumptions.

Text sources accept plain text, Markdown, CSV, TSV, and a serialized structured
IR in JSON. File sources are bounded to 16 MiB. JSON files reconstruct the
typed document rather than being treated as prose.

### Analyze a financial statement as text

```json
{"operation":"PROCESS_KNOWLEDGE_WORK","request":{"schema":"B_CORE_KNOWLEDGE_WORK_REQUEST_IR_1","request_id":"FIN-1","command":"이 재무제표를 분석하고 회계 등식을 확인해","source":{"type":"TEXT","text":"항목,2025,2026\n총자산,100,120\n총부채,40,50\n총자본,60,70","format":"CSV"},"output":{"mode":"TEXT","format":"MARKDOWN","overwrite":false},"context_tags":["finance"],"max_plan_steps":12}}
```

The response includes lexical activations, the validated `PlanIR`, the parsed
`FinancialStatementIR`, evidence-bound findings, and rendered Markdown. Balance
sheet analysis checks `assets = liabilities + equity` with scale-aligned exact
decimal arithmetic when the three totals are present.

### Write a chart to an SVG file

```json
{"operation":"PROCESS_KNOWLEDGE_WORK","request":{"schema":"B_CORE_KNOWLEDGE_WORK_REQUEST_IR_1","request_id":"CHART-1","command":"이 데이터로 선형 차트를 작성해","source":{"type":"TEXT","text":"period,value\nQ1,10\nQ2,15\nQ3,25","format":"CSV"},"document_kind":"CHART","output":{"mode":"FILE","format":"SVG","path":"D:\\B_Core_Output\\trend.svg","overwrite":true},"context_tags":["data"],"max_plan_steps":12}}
```

File output uses a staged write, validates the extension against the requested
format, and returns byte count plus SHA-256. `TEXT`, `FILE`, and `BOTH` are
distinct modes. Existing files are not replaced unless `overwrite=true`.

### Write a designed, print-ready business document

`HTML` output is a self-contained document with no network dependency. It
contains A4/Letter print rules, a cover, document map, editorial typography,
metric cards, responsive tables, themed inline SVG charts, an execution
timeline, evidence review, risks, and a decision call-to-action. Default themes
are `ACADEMIC_EDITORIAL` for papers, `EXECUTIVE_NAVY` for business plans, and
`PROPOSAL_COBALT` for business proposals. `MINIMAL_MONOCHROME` is also
available. The response echoes the resolved `design` so the visual contract is
auditable.

```json
{"operation":"PROCESS_KNOWLEDGE_WORK","request":{"schema":"B_CORE_KNOWLEDGE_WORK_REQUEST_IR_1","request_id":"PLAN-DESIGN-1","command":"투자위원회용 사업계획서를 디자인 좋게 작성해","output_language":"KOREAN","design":{"schema":"B_CORE_DOCUMENT_DESIGN_IR_1","theme":"EXECUTIVE_NAVY","page_size":"A4","brand_name":"B_CORE LAB","accent_color":"#087F6B","compact":false,"show_table_of_contents":true,"show_page_furniture":true},"output":{"mode":"BOTH","format":"HTML","path":"D:\\B_Core_Output\\business-plan.html","overwrite":true},"context_tags":["business","design"],"max_plan_steps":12}}
```

The generated HTML can be opened in a browser and printed to PDF without
changing the document data. Source-free creation deliberately leaves evidence
placeholders instead of fabricating market, financial, or research facts.

### Revise a structured plan through a new word

Inject a lexeme whose selected sense has canonical concept `revise`, then use
that word in `command`. Grounded revision markers currently include:

- `제목:` / `title:`;
- `초록:` / `abstract:`;
- `섹션 추가:` / `add section:`;
- `행 추가:` / `add row:`;
- `작업 추가:` / `add task:`;
- `위험 추가:` / `add risk:`;
- `통화:` / `currency:` and `단위:` / `unit:`.

If no requested edit can be grounded in the target IR, revision fails closed
with `REVISION_NOT_GROUNDED` rather than silently rewriting unrelated content.

## Boundaries

This API performs deterministic structure extraction, exact numeric parsing,
bounded analysis, plan generation, and grounded rendering. It does not claim
general PDF/OCR/image understanding or unrestricted scholarly authorship.
Unknown facts are not invented: a paper created without source material is a
structured draft whose factual content still requires evidence injection.

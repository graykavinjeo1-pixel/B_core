# B_Core Native OCR Roadmap

## Final objective

B_Core owns the document-vision topology, weights, corpus, inference runtime
contract and promotion evidence.  It reads Korean long-term-repair documents
without PaddleOCR, cloud vision, an LLM or an external OCR process.  The target
is not generic screenshot captioning: it is reliable recovery of text, tables,
facility names, specifications, quantities, units, prices and totals from
unseen apartment-plan formats.

## Architecture

```text
page pixels
  -> native orientation/quality head
  -> native text + table-rule + cell-junction detector
  -> geometric row/column/cell reconstruction
  -> native Korean Unicode CTC recognizer
  -> B_Core semantic facility/value compiler
  -> arithmetic and cross-page reconciliation
  -> confidence/calibration gate
  -> canonical plan facts
```

The neural layer observes pixels.  The deterministic layer validates geometry,
types and arithmetic.  The semantic layer resolves domain concepts.  None of
these layers may contain apartment-specific phrases or coordinates.

## Stages

### Initial - bootstrap

- Keep the current OCR service as the production sensor.
- Generate verified native training labels from PDF text coordinates, human
  corrections and only structurally consistent pseudo-labels.
- Train the B_Core-owned line recognizer and document detector.
- Split train/dev/test by complete document hash before creating crops.

### Middle - shadow and canary

- Run native OCR without affecting customer output.
- Compare character accuracy, critical-field exactness, table F1, numeric
  error, latency and calibration on unseen documents.
- Send uncertain disagreements to correction storage; never silently learn a
  guessed label.
- Promote a small canary only after repeated blind passes.

### Late - native primary

- Native OCR becomes primary; the temporary teacher is called only when the
  B_Core confidence/consistency gate fails.
- Distill recurring corrections into verified training examples, not lexical
  exception code.
- Export quantized native weights and keep detector/recognizer warm.

### Independent

- Remove the external OCR runtime after five consecutive independence passes.
- Continue learning from text-layer documents and administrator corrections.
- Preserve immutable receipts so a regression can roll back the model without
  rolling back accumulated verified data.

## Independence gate

- At least 500 blind documents from 40 independent template families.
- Critical quantity/unit/price exactness >= 99.7%.
- Table structure F1 >= 99.3%.
- Critical numeric error <= 0.05%.
- Native p95 latency no slower than the temporary teacher.
- Invalid Unicode output and document-specific rule count both equal zero.
- Five consecutive blind benchmark passes.

## Current implementation

- `codec.py`: stable Korean Unicode CTC reference contract.
- `component_codec.py`: compact 초성/중성/종성 composition contract for unseen syllables.
- `models.py`: B_Core-owned detector and recognizer topology.
- `build_pdf_corpus.py`: document-hash-separated verified line corpus.
- `train_recognizer.py`: GPU CTC training and owned weight receipts.
- `augment_corpus.py`: training-only Korean font/scan augmentation without evaluation leakage.
- `evaluate_recognizer.py`: document-isolated blind CER, critical-token and latency evaluation.
- `shadow_compare.py`: real scanned-page native/teacher box-aligned comparison.
- `review_sheet.py`: source crops plus competing readings for visual adjudication.
- `apply_review_labels.py`: immutable, crop-level verified labels; non-text marks are rejected.
- `mine_hard_examples.py`: document-balanced train-only failure replay without benchmark access.
- `regression_gate.py`: fail-closed four-family comparison for accuracy, critical units, Unicode and latency.
- `promotion.py`: fail-closed staged independence policy.

The production route stays unchanged until the shadow gate is satisfied.

## Measured diversified bootstrap checkpoint (2026-09-29)

`bootstrap-component-hardreplay-v7-0001` is the selected research baseline.  It
still has no production authority.

- Corpus: 15 complete documents split by document hash before cropping.
- Line crops: train 9,649, dev 1,785 and test 324 before codec admission.
- Verified scan labels: 65 Goldclass retention rows plus 29 training rows from
  a separate flowchart page.  Neither is reported as blind evidence.
- True blind scan page: 33 visually adjudicated rows from an entirely held-out
  Mudeung plan.  It is never included in training or augmentation.
- Deterministic augmentation: clean and scanner-degraded variants only for the
  train split.  A 4,320-row generated numeric/unit glyph curriculum contains no
  apartment-specific values or benchmark answers.
- Invisible PDF control characters are removed before training.  Visible text
  is preserved, while the equivalent `m²`/`㎡` and `m³`/`㎥` forms are normalized
  to the canonical data units `㎡` and `㎥`.
- Critical-token evaluation now compares the unit with the number.  Earlier
  scores that ignored a spaced unit are superseded.

Selected-checkpoint measurements:

| Evaluation set | Character | Line exact | Critical value + unit |
| --- | ---: | ---: | ---: |
| Expanded dev | 90.93% | 51.81% | 84.89% |
| Held-out text-layer test | 97.19% | 60.06% | 88.00% |
| Goldclass retention | 95.88% | 78.46% | 97.06% |
| Mudeung scanned blind | 96.85% | 87.88% | 89.47% |

The model mined 1,503 failures from train-only documents: 462 critical
value/unit rows, 577 other numeric rows, 222 long/leader rows and 242 ordinary
text rows.  Document-balanced replay improved blind critical accuracy without
reading dev/test labels.  The selected checkpoint raises the worst critical
score across the four sets to 84.89%.  Selection therefore rewards
cross-template generalization rather than one-document specialization.

The v7 selection passed the multi-benchmark regression gate against v5.  The
gate tolerates at most 0.5 percentage points of character regression and 1.0
point of line regression, permits no critical-field regression on any set,
requires the worst-set critical score to improve by at least 0.5 points, and
rejects invalid Unicode or a 25% p95 latency increase.

The model remains in bootstrap because all critical-field results are still far
below the 99.7% independence gate.  Production continues to use the current
PaddleOCR sensor.  No document-name, coordinate or phrase exception is
permitted.

## Native table detector checkpoint (2026-09-29)

`table-detector-scanaug-0003` is the selected research-only structure model.
It has 1,549,956 parameters and predicts text, horizontal rule, vertical rule
and junction masks on a 1,024-pixel canvas.

- Vector labels are admitted only when a PDF page has at least eight text-layer
  words.  This prevents a raster scan's page border from being mislabeled as
  its complete table geometry.
- The corpus contains 110 train, 45 dev and 15 test pages.  Documents are split
  by complete document SHA-256 before page selection.
- Training pages receive two deterministic photometric scan variants.  Blur,
  JPEG loss, noise and illumination change; geometry and labels do not.
- The threshold 0.65 was selected on dev only.  Test was evaluated once with
  that fixed threshold.
- A deterministic geometry compiler now joins broken collinear segments and
  reconstructs row/cell candidates.  Missing row-local vertical boundaries
  remain missing, so merged cells are not split by a global column grid.

| Evaluation set | Text F1 | Table structure F1 | Row topology F1 | Cell topology F1 | p95/page |
| --- | ---: | ---: | ---: | ---: | ---: |
| Dev, 45 pages | 89.48% | 68.28% | 76.38% | 85.49% | 4.85 ms |
| Held-out vector test, 15 pages | 87.64% | 65.93% | 72.41% | 74.23% | 7.77 ms |

Eight unseen raster pages from the Goldclass plan were also reviewed with
colored activation overlays.  Text and rule activations remained aligned and
the geometry compiler produced row/cell graphs, but that review is qualitative
and is not promotion evidence.  Verified raster pixel masks are still required.

The first owned end-to-end raster table route now runs detector -> geometry ->
row/cell crops -> component CTC recognizer -> confidence/integrity gate.  A
356 KB train-only character-composition model compares direct-row and joined
region readings; low-scoring rows stay `needs_review`.  CTC beam recovery is
limited to weak rows and only reorders tokens already proposed by the neural
recognizer.  Malformed financial punctuation and unbalanced delimiters are
rejected deterministically.

The geometry compiler now consolidates duplicate scan-rule strokes, separates
text at reconstructed row boundaries, and uses source-image ink only to refine
the detector's broad text support masks.  It does not inspect document names,
phrases or apartment-specific coordinates.  On the 65-label Goldclass page-27
retention control, an oracle crop reaches 96.04% character accuracy and 81.54%
line exactness.  The refined detector crop reaches 79.64% character accuracy
and 53.66% line exactness on the 41 labels that have a clean one-to-one region
match.  This identifies crop reconstruction as a separate bottleneck; the
control was used in training and is explicitly not blind promotion evidence.

On the fully held-out Mudeung raster page, all 33 visually adjudicated labels
received a usable detector region.  End-to-end detector-crop recognition
reached 100% character accuracy and 100% line exactness.  The former
`타입` -> `타압` error was recovered by a scale-aware padding retry only when
both neural confidence and the train-only character language score improved.
This page and its labels were not used for detector or recognizer training.  It
is valid blind evidence, but one document is far below the
500-document/40-family promotion gate.

On 300 dpi Goldclass pages 18 and 30, the warmed research path took 1.08 s and
0.84 s respectively, including 49 row decisions.  Exact-width batching reduced
latency by roughly 17-22% while changing zero selected row strings.  A faster
padded-width batch was rejected because bidirectional-GRU padding changed 22 of
49 selected strings.  This is a research timing result, not a production SLA.

After source-ink line splitting, those pages expose 89 physical text-line
decisions rather than merging stacked rows.  Fail-closed beam eligibility now
forbids recovery from blank and one-mark crops; this removed four false
acceptances (`30`, `0.`, `2.`, and another noise row) in the two-page control.
The current richer path takes 1.43 s and 1.21 s on those pages, with only 12 and
5 beam searches.  It is safer but still requires latency work before promotion.

Cells and rows now carry normalized numeric tokens and integrity status.  A
typed, language-independent arithmetic gate validates quantity × unit price ×
rate against amount and validates additive totals.  It never guesses column
roles from Korean text; the table-schema compiler must supply those roles, so
arithmetic can reject corrupted facts without adding phrase exceptions.

The first source-pixel rule-fusion ablation was rejected.  It improved the
first Mudeung scan structure check (row F1 40.00% -> 85.71%, cell F1 28.57% ->
72.46%) but collapsed the held-out vector topology.  Source-rule guessing is
therefore absent from the canonical runtime and evaluators.  The initial
Mudeung structure result remains recorded as blind evidence; after inspecting
that result the annotation is development-only for later experiments.

A label-derived rule-erasure training candidate selectively faded and broke
table rules while preserving the unchanged geometry target.  It was rejected:
held-out vector row/cell F1
fell to 53.97%/59.41%, and the raster development page did not improve.  The
canonical training pipeline was restored to photometric-only augmentation.

A second, narrower experiment now requires a physically continuous thin stroke
plus a connected horizontal/vertical grid before source pixels may recover a
scan rule.  It replaces neural fragments only inside the proven grid.  On the
Mudeung development page this reached 100% row F1 and 98.46% cell F1.  Generic
geometry cleanup (joining adjacent collinear fragments and dropping only short
empty phantom rows) also improved vector dev topology to 76.38%/85.49% while
holding held-out vector test at 72.41%/74.23%.  Scan recovery is still not a
production path: applying it indiscriminately to vector pages fails, and more
independent scan families are required to validate the page-type gate.

The owned table reader normalizes 0/90/180/270-degree page orientation before
recognition.  Orientation is decided independently for every page; a document
can legitimately mix portrait, landscape and upside-down scans.  A former
document-wide cache was removed after it incorrectly reused one page's angle
for later pages.  Native visual scoring is now cross-checked by an already
installed local document-orientation observer.  The observer cannot download a
model or own text facts.  Agreement is accepted, strong axis correction is
allowed, and unresolved disagreement remains review evidence.

## Multi-algorithm evidence compiler (2026-09-30)

The runtime no longer assumes that one detector, preprocessing recipe or OCR
engine is uniformly best for every document form.  It operates on evidence
spans rather than voting on complete OCR outputs:

1. trustworthy embedded PDF text is used directly when available;
2. the owned detector/geometry/recognizer route reads native raster rows;
3. a separately implemented local detector/recognizer may propose additional
   boxes only on hard scan pages;
4. B_Core rereads the exact proposed glyph pixels with its owned recognizer;
5. only identical non-whitespace content with preserved numeric facts and an
   owned confidence of at least 0.80 is admitted;
6. coordinate-equivalent evidence is deduplicated, and a verified span nested
   inside an accepted row is retained as non-additive support;
7. review-only or support-only evidence has no fact authority;
8. independently verified numeric disagreement quarantines the native row
   instead of selecting either value.

Multiple visual variants of the same model are explicitly not counted as
independent algorithms.  They may improve recall but cannot create additional
fact authority.  Every accepted item records its coordinates, granularity,
provenance and authority state.  `audit_evidence_compilation.py` fails a receipt
if review evidence owns facts, nested support is counted twice, active evidence
is duplicated, independent provenance is missing, or a numeric-conflict row
remains accepted.

Measured integration receipts:

| Document/page | Native accepted rows | Observer spans | B_Core-verified spans | Final primary fact evidence | Non-additive support | Review only | Evidence audit |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| Manwolsan tunnel technical drawing p60 | 10 | 283 | 134 | 107 | 37 | 23 | PASS |
| Goldclass long-term-repair scan p35 | 23 | 263 | 224 | 116 | 129 | 18 | PASS |
| Mudeung long-term-repair scan p35 | 0 | 109 | 107 | 107 | 0 | 11 | PASS |

The Mudeung result demonstrates the intended separation: the owned recognizer
could read 107 small spans once another geometry path located them, even though
the native whole-row route accepted none.  Detector diversity, not blind text
voting, supplied the missing coverage.

The locally installed PP-OCRv5 server detector was also tested as a third
proposal path.  At 1,280 pixels it failed a 1.76 GB contiguous GPU allocation;
at 640 pixels it still failed an 0.89 GB allocation and consumed about 42
seconds without producing evidence.  It is therefore rejected from the
canonical route.  The mobile detector remains the sparse observer until a
third path proves both new verified coverage and acceptable resource use.

The 121-page Yongsan Richville run reached 91.90% text F1, 92.17%
table-mask F1, 95.65% row topology and 94.54% cell topology, with 8.32 ms p95
detector latency per page.  A provenance audit then found that table-corpus-v3
had already used 20 pages from the same document in the detector's train
split.  This run is therefore retained only as a training-family retention
control and is not evidence of blind generalization.  Its selection record
and evaluation receipt explicitly carry that correction; the promotion gate
remains unchanged and still requires genuinely unseen document families.

The reader now gates scan-only geometry recovery with the same document-
independent text-layer evidence used by corpus admission.  Goldclass page 27
has zero vector words and enabled recovery; Yongsan page 20 has 168 vector
words and kept the neural/vector path.  The gate is covered by synthetic grid
and threshold-boundary tests, bringing the native OCR contract suite to 21
passing tests.  This removes the prior risk of applying scan pixel recovery to
vector pages, where the ablation proved it would be destructive.

A provenance and label audit added two previously unseen long-term-repair
document families.  The first evaluation exposed a serious ground-truth bug:
the corpus builder treated every PDF vector edge as a table rule, so slide
callouts and graph-paper backgrounds became fictitious tables.  The builder
now derives rules only from two-dimensional table-cell grids, rejects page
frames and one-row callouts, and rejects implausible backgrounds above 1,000
cells.  The corrected 40-page benchmark scored 62.77% row topology and 64.71%
cell topology.  The practice-guideline family scored 70.48%/67.27%, while the
decorative training-slide family scored 24.07%/36.65%, exposing a real false-
positive weakness on presentation graphics.

The first diversity retrain (`table-detector-diverse-v4-0001`) was rejected.
At the dev-topology threshold it regressed the prior test from 72.41% to
70.00% row F1 and from 74.23% to 69.23% cell F1, without materially fixing the
held-out slide family.  The selected research checkpoint therefore remains
`table-detector-scanaug-0003`; no loss/step-only improvement was promoted.

The table geometry compiler no longer pairs every adjacent horizontal line on
the page.  It first builds connected horizontal/vertical rule components and
only promotes a component when it contains at least two row bands and a real
column split.  Stacked callout boxes and unrelated slide frames remain layout
elements instead of becoming table cells.  On the corrected 35-page held-out
set this raised the selected checkpoint's row topology from 62.77% to 68.29%
and cell topology from 64.71% to 65.00%, while 25 contract tests passed.

The corrected-label retrain (`table-detector-gridlabels-v5-0001`) was also
rejected.  At its dev-selected threshold it reached 56.79% row and 62.94% cell
topology on the same held-out set, below the selected checkpoint's 68.29% and
65.00%.  A page-level visual audit also found that held-out slide page 107
contains a genuine rasterized table that the vector-only ground truth marks as
empty.  The current figures are therefore provisional lower bounds; raster
tables must be manually adjudicated rather than silently used as negative
training examples.

A versioned visual-adjudication receipt now separates model errors from label
errors.  Three pages with visually confirmed tables but empty vector masks are
explicitly excluded from scoring; confirmed workflow diagrams, graph-paper
backgrounds and photo-caption cards remain negative controls.  On the 33
scorable held-out pages, the selected checkpoint reaches 76.71% row and 75.58%
cell topology.  The v5 candidate reaches 63.89%/72.51%, reinforcing its
rejection.  This exclusion mechanism does not turn detector proposals into
ground truth and does not add any production document-specific rule.

A structural-loss candidate (`table-detector-axisprofile-v6-0001`) added
page-scale horizontal/vertical max-projection supervision.  It looked useful
on adjudicated dev (77.33% row, 82.66% cell) but collapsed on the untouched
adjudicated test to 53.13%/59.21%, versus the selected model's 76.71%/75.58%.
It was rejected.  This closes the simple axis-projection hypothesis: global
line presence is not a substitute for learning connected grid topology.

The next work is to create independently checked raster table masks from new
document families, measure ruled and borderless layouts separately, and teach
the table-schema compiler to supply typed arithmetic roles without lexical
exceptions.  Table promotion remains blocked until structure F1 reaches 99.3%
across the full blind-document gate.

## Diversity and independent scan tranche (2026-09-29)

A geometry-only profiler now measures source diversity before training.  The
first new pool contains 13 document families spanning vector text, ruled
tables, mixed vector/raster pages, landscape forms and fully scanned
casebooks.  Large documents are sampled deterministically across their whole
page range rather than taking only their opening pages.  The line corpus adds
3,319 train, 338 dev and 294 untouched vector-test crops; scan pages without a
text layer are never admitted as automatic truth.

Two visually adjudicated pages from the entirely scanned community-housing
consultation casebook provide 48 new blind lines.  The selected v7 recognizer
reaches 96.76%/97.81% character accuracy on pages 30/90, but only
24.00%/47.83% full-line exactness.  These pages remain outside training.
Twenty rows from a separate train-split scanned announcement were manually
verified for adaptation experiments.

Three continuation candidates were not promoted.  v8 mixed all new vector
families at once and forgot prior scan behavior.  v9 narrowed the curriculum
but changed BatchNorm running statistics.  v10 froze those statistics and
improved Goldclass, Mudeung and casebook page 30, yet still regressed legacy
dev/test and casebook page 90 as a standalone checkpoint.  The selected v7
model remains unchanged.  A diagnostic v7/v10 confidence-plus-language
selector improved or preserved the checked scan families, but it is not a
runtime path until more independent scan documents validate routing.

The table detector was also evaluated, without threshold retuning, on a new
10-page vector specification family.  It reached 98.11% row topology and
92.50% cell topology.  A second new dev family reached 66.67%/72.73%, showing
that document-family coverage remains the limiting factor rather than a single
global detector threshold.

The geometry compiler now recovers disconnected full-width merged header rows
only when a proven two-dimensional grid, matching span/spacing, actual text
ink and a surviving outer border all agree.  This restored the Mudeung row
topology to 100% and cell topology to 98.46% without changing adjudicated dev
(80.00%/86.61%) or held-out test (76.71%/76.74%) topology.  Thirty-two native
OCR contract tests pass; source-specific runtime rules remain zero.

The train-split scanned announcement now contributes 88 fully visual-verified
rows across three pages, including prose, dates, URLs, circled list markers
and short table cells. The two consultation-casebook pages remain untouched
blind tests. A balanced continuation candidate (v11) still exhibited
catastrophic interference, so it was rejected rather than promoted.

Recognizer training now supports frozen-teacher sequence distillation over
valid CTC time steps. This is a document-independent continual-learning
safety mechanism: each adapting student is penalized when it erases the
mature baseline distribution, while new verified labels remain the supervised
signal. The first LwF candidate (v12) reduced some drift but did not satisfy
the zero-regression gate, especially on casebook page 90, and was also
rejected. The selected checkpoint remains v7. This establishes that more
independent scan families, not repeated training on one announcement, are
required before a scan expert can replace or join the runtime path. Thirty-
three native OCR contract tests now pass.

An additional 95 rows were visually adjudicated from a scanned long-term-
repair facility-information table. This brings the current scan adaptation
pool to 183 verified rows across four pages and two independent document
families. The resulting multi-scan LwF candidate (v13) preserved Mudeung and
Goldclass behavior but reduced legacy dev full-line accuracy to 48.53%, legacy
test to 58.20%, and untouched casebook page 90 to 34.78%. It was rejected; v7
remains the selected recognizer. This confirms that adding verified pages is
useful evidence but does not justify replacing the mature checkpoint with a
single continually fine-tuned model.

The next corpus expansion is now driven by train-only model disagreement.
The active-learning queue compares multiple compatible checkpoints, ranks
functional prediction disagreement plus uncertainty, and emits visual review
sheets without reading any target text. Dev, test and blind-eligible records
are rejected at the loader boundary. This concentrates visual adjudication on
new combinations the models actually distinguish, while keeping all blind
benchmarks outside corpus construction.

A further 100 visually verified table-cell crops were added from page 62 of
the long-term-repair guide, focused on household counts, floor area, floor
counts, corridor type and elevator quantities. This lifts the scan adaptation
pool to 283 verified rows across five pages. A separate v14 table-cell expert
was trained without replacing v7. It improved the independent Mudeung table
from 87.88% to 90.91% line exactness and 89.47% to 94.74% critical-value
exactness. On a newly adjudicated 108-cell blind page from the Bongseon Geumho
plan, it improved character accuracy from 93.81% to 94.07% and line exactness
from 78.70% to 79.63%, with critical fields remaining 100% exact.

This is useful evidence for sparse specialization, but not enough for runtime
routing. A new fail-closed specialist gate requires at least 12 independent
table documents, six template families, non-regressing line/critical metrics,
a measured router false-positive rate below 0.5%, and three consecutive blind
passes. v14 is therefore research-shadow-only and the global v7 selection is
unchanged. Thirty-eight native OCR contract tests now pass.

## Blind-boundary correction and structural table reading (2026-09-29)

The vector table-cell benchmark now resolves every candidate PDF hash against
all training manifests recorded by both the baseline and candidate checkpoint.
Training-document collisions and duplicate input hashes are excluded before a
single crop is written. This audit found that the earlier Bongseon and Yongsan
vector-cell comparison was useful as a regression control but was not blind:
both hashes occur in the v7 training corpus. The research receipts and active
selection record have been corrected rather than preserving the inflated
claim.

A replacement benchmark contains 604 tight vector-text rows from two genuinely
unused long-term-repair template documents. On this independent set v7 reached
95.66% character accuracy, 85.93% full-line exactness and 93.58% critical-field
exactness. v14 fell to 92.78%, 72.52% and 68.58%, respectively. The fail-closed
router correctly selected v14 zero times. v14 is rejected for runtime use and
retained only as a research artifact; the global v7 checkpoint remains
selected.

The independent errors exposed a reusable structural ambiguity rather than a
document-specific phrase: numeric columns repeatedly rendered zero as Latin
`O`. The table compiler now assigns stable component/row/column indices, and a
numeric-column consensus can normalize that glyph only when at least three
same-column readings are numeric-like and at least two are already unambiguous
numbers. No document name, phrase, unit or answer label participates. Across
the independent 604-cell set this raises v7 to 96.48% character accuracy,
88.08% line exactness and 97.97% critical-field exactness. The first independent
document improves on all three measures and the second remains unchanged. The
older 1,638-cell regression control also improves rather than regresses.

The native OCR contract suite now has 44 direct tests and 57 related pipeline
tests passing. The next evidence tranche must add new scanned and vector-table
document families, with hash exclusion enabled from corpus construction, before
any neural specialist or production-authority promotion is reconsidered.

A low-contrast continuation candidate (`bootstrap-component-lowcontrast-v15-0001`)
was trained only from train-split hard examples with deterministic faint-text
augmentation, frozen BatchNorm statistics and v7 replay distillation. It was
rejected. Independent vector-cell line exactness fell from 85.93% to 81.46%,
legacy dev fell from 51.81% to 48.13%, Goldclass fell from 78.46% to 64.62%,
and consultation page 90 fell from 47.83% to 30.43%. A dual-input autocontrast
probe also produced only a negligible net gain while introducing new errors.
The selected v7 checkpoint remains unchanged. Faint small Korean table text
therefore requires a representation or resolution improvement, not broader
synthetic degradation or a second inference pass.

## Table pipeline and multi-document generalization tranche (2026-09-29)

The production-equivalent table-cell evaluator now records greedy recognition,
target-free CTC beam selection and structural normalization separately.  On 604
independent cells, the selected v7 recognizer improved from 95.69% character,
86.09% line and 93.58% critical-value accuracy to 97.27%, 90.89% and 98.99%
after beam selection and a general numeric-token O/0 correction.  The older
1,638-cell regression control also improved to 98.13%, 93.65% and 97.59%.
The O/0 correction only acts inside independently delimited digit-containing
numeric tokens; Korean words, standalone O and opaque identifiers remain
unchanged.  The same post-process was positive or neutral on every checked
non-table set.

A train-only word lexicon was built and tested as a second recovery signal. It
regressed the independent table benchmark even under conservative frequency
and language-score gates, so it is not part of runtime. Isolated word
frequency is not sufficient context for reliable OCR correction.

The corpus boundary found and fixed a training harness defect: additional
cross-corpus `--source` inputs had replaced, rather than appended to, the
57,799 replay examples. The corrected loader always preserves ordinary
manifests, assigns every sample an explicit replay/adaptation role and applies
frozen-teacher distillation only to replay samples. Sixty-six related contract
tests pass after this change.

The first correct domain-expansion candidates, v16 and v17, improved table and
new-guide metrics but cut consultation-page-90 critical accuracy from 80% to
40%. Both were rejected globally. v17 demonstrated that limiting distillation
to replay samples is structurally correct but does not by itself remove this
cross-family interference.

A new document-hash split covers 12 additional PDFs: eight train, two dev and
two completely unused test documents. It spans long-term-repair tables, legal
prose, bid notices, work specifications, a user manual and a research report.
From 120 stratified pages it produced 1,747 train, 448 dev and 688 test crops;
unsupported labels are excluded rather than coerced. The selected v7 baseline
scores 85.99% character, 55.00% line and 67.92% critical accuracy on the new
hard test. Diverse replay candidate v18 improves these to 86.48%, 57.94% and
68.26%, but still regresses consultation-page-90 and Goldclass, so it is not
promoted.

Checkpoint interpolation showed the regression boundary is sharp rather than
gradual: 1% of the v18 adaptation delta preserves the p90 critical score but
provides essentially no useful new-document gain; 5% already reduces p90
critical accuracy from 80% to 60%, and 25% reduces it to 40%. This rules out a
simple model-soup solution. A new global replacement gate now rejects average
improvements whenever any independent document family loses line or critical
accuracy. The selected checkpoint remains v7; all v16-v19 artifacts have no
production authority.

## Causal delta isolation and conservative sparse routing (2026-09-29)

Module-level checkpoint composition isolated the diverse candidate's behavior.
Replacing only v7's visual encoder with v18's visual encoder retained most of
the new-document gain, but it also reproduced the major consultation-page
regression. Context and output-head replacements produced smaller interference.
This shows that a single global checkpoint is still unsafe and that the useful
adaptation is primarily visual rather than a new vocabulary or decoder effect.

A target-free conservative router now exists as an evaluation-only path. It
never reads the target label and rejects specialist output when numeric tokens
change, leading list/table structure disappears, text integrity fails, the
train-only character language score falls, or the specialist confidence does
not exceed the v7 anchor by a fixed margin. The first 1.0% margin passed one
blind set but exceeded the false-positive gate on a second set. That set was
converted to development evidence; a 1.25% margin was then locked before any
further blind evaluation.

With the margin frozen at 1.25%, three consecutive blind runs covered 1,210
usable lines from 12 independent documents and eight template families. The
routed system reduced edit errors from 1,941 to 1,931, improved exact lines
from 513 to 520, preserved all 244 correct critical-field readings and made one
harmful selection (0.0826%). Aggregate character accuracy rose by 0.0313
percentage points and line exactness by 0.5785 points. No document names,
phrases, answer labels or target text participate in routing.

This passes the generalization-evidence gate but not the production gate. The
router remains `research_shadow_only`: the complete raster/page/table path and
real dual-model latency still require measurement. v7 remains the only selected
checkpoint and production authority remains false. The promotion policy now
separates research evidence from production readiness so a benchmark gain
cannot silently activate a new runtime path.

Latency was then measured on real raster line crops rather than inferred from
parameter counts. Sequential anchor-plus-specialist inference cost 2.01 times
the v7 anchor. Because v20 differs only in `visual.*`, a fused implementation
now stores one shared GRU/output head and evaluates both visual streams in one
combined batch. Its first exact-width implementation reduced the ratio to
1.38, but still exceeded the 1.15 canary gate. Naive image padding changed at
least one decoded line on a 512-line run and was rejected. Width quantization
was also rejected: every tested quantum introduced a critical numeric-field
regression even when aggregate throughput improved.

The accepted batching method instead carries each crop's real width through
the visual network and masks artificial padded regions after every convolution
and residual substep. Across all three frozen blind sets (1,210 lines), batch
size 12 and width bins of 32 preserved every anchor and specialist decoded
string. It accelerated the selected v7 anchor by 1.45-1.95 times and reduced
the fused specialist overhead to 6.21-13.69%, passing the 15% latency gate on
every set. The production-equivalent native crop reader now uses this masked
batch path.

The masked batch reader was then exercised through the complete detector,
geometry, region, row, beam and structural-normalization pipeline. Two
long-term-repair table pages and one scanned consultation-casebook page covered
487 text regions, 100 rows and 152 cells. They produced zero region, row or
cell text differences against the one-crop reference path. The conservative
warmed table-page comparison improved total page latency by 1.68 times, and
the scanned page improved by 2.04 times. This closes the anchor batching
blocker. The fused routed specialist was also connected to the complete raster,
detector, geometry, region, row, beam and structural-validation path. Across
three raster pages it made 566 routing decisions and selected five
specialist/recovery outputs. A warmed borderless table page completed in
907.94 ms and retained the same 17 accepted and four fail-closed rows as the
anchor path. This proves integration and latency, but does not authorize v20:
too few changed selections have been independently adjudicated against the
visible page. A separate promotion gate now requires at least 50 visually
adjudicated end-to-end changes, so the route remains research-shadow-only.

## Borderless scan-table reconstruction (2026-09-29)

Visual audit of a full long-term-repair summary page found that the apparent
cell failures were primarily geometric, not character recognition failures.
The source table intentionally had horizontal rules but no vertical rules;
the previous scan recovery chained aligned glyph strokes into 25 false
vertical lines and emitted merged or phantom cells. A source-continuity guard
now requires a vertical rule to remain dark through its claimed height. On the
audited page this reduced 29 raw candidates to the two real page-border rules
and rejected 27 glyph-stroke chains. The ordinary consultation-casebook scan
retained identical geometry and row text.

When a table has no trustworthy vertical grid, the pipeline no longer invents
cells. It reconstructs repeated numeric rows from ordinal/year progression,
visual row spacing and the modal token count. It then discovers repeated
arithmetic relations between columns from the page itself. No apartment name,
heading or answer label is used. Three tables and 21 data rows were recovered
on the audited page. Seventeen rows matched the visible source and passed both
count and arithmetic checks; all four OCR-damaged rows were fail-closed as
`needs_review` because of a missing value or an arithmetic mismatch. The
masked batch path reproduced every text and approval decision and was 2.21
times faster than the one-crop reference. Eighty-five related contract tests
pass after the change.

## Diverse visible-page supervision (2026-09-29)

Seven visually adjudicated pages from four unrelated document families now
provide 277 real line labels and 1,108 generated scan variants. These labels
are training-only and cannot be counted as blind evidence. The first full-model
candidate trained on them improved frozen-blind aggregate edit errors from
1,926 to 1,686, exact lines from 513 to 568 and critical values from 244 to
254 across 12 documents. It produced no document-level character or line
regression, but lost three previously correct critical values in two documents.
That is sufficient to reject global replacement.

A confidence-only research router preserved all 244 anchor critical readings
and improved exact lines from 513 to 526. It still made two harmful selections,
so it remains shadow-only. Visual-only and context/output checkpoint ablations
also regressed, while increasing whole-sample numeric weights made critical
generalization worse. The next stability experiment therefore acts only on
teacher time positions that emit numeric literal tokens instead of freezing an
entire numeric line.

The borderless reconstruction output is also compiled into a generic,
source-scoped evidence contract. Rows that pass structural and arithmetic
validation are separated from a fail-closed review queue. No apartment-specific
field names are inferred at this stage, so downstream plan generation cannot
mistake a geometric reconstruction for a verified domain fact.

## Sparse numeric anchor and arithmetic-proven recovery (2026-09-30)

The diverse v21 recognizer is still unsafe as a global replacement, but its
open-text gains can now be evaluated through a separate sparse numeric path.
The path reads every crop once with v21, wakes the selected v7 anchor only for
low-confidence visible numeric facts and obtains two deterministic alternate
visual readings only when those models disagree. The router may choose only an
already visible v21 or v7 fact set; alternate views vote but can never inject a
third value. A numeric-fact cardinality guard blocks a visually popular answer
that adds or removes a number outside a typed numeric column.

On five existing multi-family sets, the route matched or improved the v21
primary's edit count while preserving every v21 numeric fact. A first newly
frozen document exposed one added-number failure before the cardinality guard;
that document was immediately converted to development evidence. A second
clean, hash-frozen government document supplied 156 usable lines. It preserved
49 of 51 numeric facts, made no model-changing selection and added 3.9% runtime
overhead. This is a clean non-regression pass, not evidence that disagreement
adjudication generalizes: the new set contained zero eligible disagreements.
The sparse route therefore remains research-only and v7 remains selected.

A later frozen 283-line safety-training evaluation did contain four numeric
disagreements and caught a second failure: repeated visual votes preferred
`199` over the primary's `109` while the label was `100`. Neither model had a
structural reason to decide that isolated cell. The set was converted to
development evidence and the router now forbids replacing a standalone number
unless an independent table contract has already typed the position as
numeric. Re-running six sets (1,684 lines) then preserved every primary output
and numeric fact, but selected the anchor zero times. That is safe but not
useful, so the dual-model numeric route stays disabled rather than adding
latency without demonstrated gain. Future reactivation requires typed table
structure and a new frozen set containing real disagreements.

Full-page table execution now keeps complete row candidates with repairable
numeric punctuation until structural validation instead of discarding the
whole row early. Repeated table arithmetic can canonicalize dot/comma grouping
only when exactly one candidate satisfies every learned relation. It may also
repair one already visible numeric token when exactly one one-character edit
satisfies all relations; it never fills a missing column. On Goldclass page 30
this changed the generic borderless evidence result from 18 approved rows and
three reviews, through 20/1 after punctuation recovery, to 21/0 after candidate
ordering and the unique one-edit arithmetic proof. The 200-DPI end-to-end run
completed in 1.71 seconds. All 97 runtime tests pass. These recoveries are
generic table-integrity operations, not apartment-, phrase- or row-specific
rules, and every repair is retained in evidence provenance.

## Raster orientation and general text-line recovery (2026-09-30)

A previously unused six-page hospital-budget slide deck exposed a different
first failure layer. It has no PDF text layer and its visible Korean is clear,
but the table-oriented detector fragmented icons and decorative panels into up
to 141 regions. Worse, the old OCR-only orientation vote rotated the upright
cover by 270 degrees and cached that error for the document. The resulting
gibberish was not a recognizer vocabulary problem.

The scan path now measures wide horizontal text-line support before accepting
an orientation. On the cover the upright and upside-down candidates had 0.984
horizontal coverage while the 90/270-degree candidates had zero; the existing
language score then distinguished 0 from 180 degrees. The selected orientation
changed from the incorrect 270 degrees to 0 degrees. The cover consequently
reads `기초 학습자료` and `병원 예산 관리 완전정복` as two complete rows.

A language-independent scan text detector was added for pages where the frozen
detector's useful-reading ratio is below 60% or median confidence is below 80%.
It removes page frames and photographic masses by connected geometry, groups
remaining ink into horizontal row bands and retains the learned detector for
table rules. It does not use Korean words, document names or expected answers.
On the slide body it recovered the visible headings and budget bullet lines
instead of detector fragments. On consultation-casebook page 90, whose learned
path already had a 100% useful-reading ratio and 97.2% median confidence, the
automatic gate retained the faster learned path and its 23 rows. Thus the CV
fallback is sparse rather than an unconditional second detector pass.

Seventeen rows from two visually checked slide pages are now immutable
training-only evidence. Tight-crop evaluation shows the selected v7 recognizer
already reaches 96% character accuracy on the 15-line body; the dominant loss
was layout and cropping. Additional neural training is therefore deferred
until more scan families are visually labelled. One hundred related runtime
tests pass. The service and selected v7 checkpoint are otherwise unchanged.

## B_Core-only component geometry and scan fact guard (2026-09-30)

The production research path now defaults the independent OCR observer to
`off`. Teacher/observer modes remain explicitly callable for corpus research,
but an ordinary page run no longer loads or calls an external OCR model.

A second B_Core-owned scan detector now clusters connected glyph components by
their visual centerline and height, then emits direct text regions without
passing them through the global row projection that previously merged tightly
stacked table lines. On Mudeung page 35 the old scan fallback produced 65
over-tall regions and zero owned-model consensus approvals. The component path
produced 141 regions, 37 exact high-confidence approvals between the two
B_Core recognizers, 37 text rows and 58 table cells. The external OCR observer
was disabled for the run. Hard-scan `auto` routing now selects this component
path; easy high-confidence scans retain the learned detector.

The same path was checked on a visually unrelated decorative hospital-training
cover. It recovered the two visible titles as full rows, but also exposed a
false low-confidence reading (`A7 년`) from decorative footer graphics. Scan
rows therefore now require 0.80 confidence to own facts from a single reading,
while an exact match from independent B_Core-owned views may still promote a
correct row through the evidence compiler. Re-running the cover preserved
`기초 학습자료` and `병원 예산관리완전정복` and quarantined the decorative false
reading. The run used component geometry automatically and external OCR
activation was false.

This is a precision and geometry improvement, not a 99.7% completion claim.
The independence target is now explicitly measured with external OCR calls at
zero. Context may re-rank candidates only when they remain visually grounded;
numeric, date, amount and identifier values may not be invented from language
plausibility. They require owned visual candidates plus structural or
arithmetic confirmation. All 191 native OCR contract/regression tests pass.

The hard-scan component path now observes three same-coordinate pixel views:
the original raster, local-contrast normalization and unsharp interpolation.
Original geometry is never discarded. A region visible only after filtering
must occur in both enhanced views before it is admitted; a single-filter
artifact remains rejected. On Mudeung page 35, all 141 original regions were
preserved and ten single-view artifacts were rejected, with no change to the
33 authoritative rows or four review rows. On the unrelated hospital cover,
five faint regions gained two-view support while 209 one-view enhancement
artifacts were rejected.

That cover also exposed a second failure mode: CTC beam search raised a weak
decorative reading from 0.749 visual confidence to a linguistically plausible
0.941 (`총 낸`). A scan beam may no longer own a fact unless a non-beam visual
reading independently meets the 0.80 scan floor. The two true titles remain
authoritative through owned-model agreement, while both decorative readings
are now review-only. This is the first explicit boundary between document
context as a candidate ranker and document context as an unsafe fact inventor.

The recognizer now exposes an auditable N-best list for a row. Every entry
must be reachable through an owned CTC visual path and retains separate
acoustic, language and combined scores. The existing top-1 behavior is
unchanged. A document-context ranker has been added as an isolated prototype:
it can only reorder those pixel-grounded candidates, rejects alternatives
outside a narrow visual-confidence margin, and cannot change a visible number,
amount, unit-bearing value, identifier or structural marker. Tests prove that
even overwhelming repetition elsewhere in the document cannot turn
`56,000,000` into `58,000,000` or rescue a weak visual candidate. This is the
safe substrate for later layout- and meaning-aware document reranking; it is
not yet enabled to change production facts until blind-family evaluation shows
a net gain with zero critical-field regressions.

The independence gate can no longer be passed by rejecting most difficult
text and reporting high accuracy on the remainder. In addition to 99.7%
character and critical-field accuracy, it now requires at least 99.5% text
detection recall and precision, at least 98% authoritative evidence coverage,
zero authoritative false positives, zero external OCR invocations and five
consecutive blind passes over the existing multi-family coverage floor. All
191 native OCR contract/regression tests pass.

## Context N-best and high-resolution recognizer experiment (2026-09-30)

The frozen v8 blind set confirms that document context is useful only after a
strong visual candidate generator exists. On all 619 supported blind lines,
the selected v7 recognizer reached 94.1270% greedy character accuracy. A wide
32-candidate visual beam raised the oracle ceiling to 96.0260%; conservative
document-context ranking reached 94.1656%, with 44 improvements and six
regressions among 78 changed rows. Therefore no context selector can reach the
99.7% target while the correct transcription is absent from roughly one third
of row candidate sets.

A larger B_Core-owned recognizer was added as an isolated research topology
(14,931,586 parameters versus 2,834,354 in the baseline). It keeps the same
component codec and auditable CTC output but increases visual and sequence
capacity. Under the currently shared GPU, a 1,024-sample, two-epoch transfer
from the frozen v7 B_Core recognizer completed without external OCR. Blind-64
character accuracy improved from 37.294% in the undistilled sanity run to
44.683%; this proves the training path works but remains severe underfit and is
not a production candidate. The selected v7 checkpoint and service routing are
unchanged.

The next target is not an unconstrained language correction layer. It is a
multi-view visual candidate union followed by generic document-structure and
semantic consistency scoring. Context may resolve a glyph ambiguity only when
the owned visual paths already support every resulting character. Critical
facts continue to require visual and structural confirmation; a plausible
sentence can never overwrite an unsupported amount, date, quantity or ID.

## Residual visual refinement and layout context (2026-09-30)

The frozen baseline now has an additive refinement topology whose initial
forward pass is numerically identical to its parent. Only the residual BiGRU
and zero-initialized correction head are trainable, so broader Korean and
mixed-script curricula can add visual distinctions without rewriting the
mature visual stem. The strongest mixed-script endpoint reached 96.9287%
greedy character accuracy, but its critical-value exact rate regressed. It is
therefore not eligible for production promotion.

Linear interpolation between the general-language v60 anchor and the
mixed-script v61 endpoint exposed a better Pareto point. A checkpoint metadata
bug that labelled refinement weights as the legacy topology was found before
selection, fixed, and covered by regression tests. With the executable
topology preserved, alpha 0.50 reached 96.9011% greedy character accuracy,
56.22% line exactness and 87.3786% critical-value exactness. Its eight-candidate
visual oracle reached 98.2552%, confirming that the remaining bottleneck is now
substantially candidate selection as well as visual recognition. This is still
below the 99.7% independence gate and is not deployed.

Document context now distinguishes whole-page repetition from spatially nearby
same-column rows using only bounding-box geometry and accepted B_Core-owned
readings. Nearby layout evidence can receive a stronger ranking weight, but it
still cannot synthesize text or alter a visible numeric/date/amount/identifier
fact. On the same 619-line development set, layout-aware context raised
character accuracy from 96.9177% to 96.9232% with the critical metric unchanged.
The small gain shows that hand-weighted n-grams are not sufficient for VLM-like
document understanding. The next research component is a learned, generic
candidate ranker over visual scores, row geometry, table/heading roles and
document context. Its output remains restricted to owned visual candidates.

The generic learned-ranker follow-up separated the seven-document training
observations from all five v8 source documents. A linear visual-plus-layout
ranker reached 96.9177% on the disjoint documents, versus 96.9011% greedy.
Adding contrastive span features or a nonlinear pairwise MLP did not improve
generalization. The learned ranker is therefore retained as a research result,
not as an enabled correction path.

Candidate-generation changes were also isolated. Expanding the owned max-path
beam from eight to 32 candidates raised the oracle ceiling to 98.5854% and the
critical-value oracle to 94.1748%, but still left the 99.7% target unreachable.
A proper CTC prefix beam and a fused prefix/max-path decoder added almost no
oracle coverage and cost substantially more time. Raising input height from 48
to 64 pixels without matching training collapsed accuracy to 64.3219%. None of
these variants is selected.

A separate visual-residual topology was then added behind the mature image
encoder. Its zero-correction initialization is numerically identical to the
parent, and the optimizer owns only the two new visual modules. A 1,024-sample
sanity adaptation at full step retained 96.8901% character accuracy but reduced
critical-value exactness from 87.3786% to 82.5243%. A 5% interpolation produced
96.9177% character accuracy while reducing line exactness and oracle coverage;
larger interpolation weights regressed further. The topology is therefore not
promoted. This confirms that VLM-style context and added visual capacity are
useful design directions, but neither a hand-weighted context score nor a tiny
residual adaptation constitutes document understanding.

All 202 native OCR contract/regression tests pass. The v8 set has now been used
repeatedly for development and cannot serve as the final untouched promotion
proof; new document-family blind sets remain required.

## Hybrid correction contract and review UX (2026-09-30)

A target-free failure analyzer now separates visual candidate coverage from
candidate ranking. On the 619-line v8 development set, the regenerated
wide-32 candidate pool reached 98.5579% oracle character accuracy while 119
lines still lacked an exact target candidate. The largest residual class was
Hangul-to-Hangul substitution (164 edits), followed by punctuation/symbol
deletion (67 edits), Latin-to-Latin substitution (50), punctuation/symbol
substitution (49) and Latin deletion (35). These counts are diagnostic; they
do not authorize language-only replacement of pixels.

The structural solver now discovers repeated addition, multiplication and
percentage-product equations without fixed headers or column numbers. A value
may be repaired only when a one-character visible numeric near-match is the
unique assignment satisfying every compatible equation. Existing additive
roles remain backward compatible. The schema-free ruled-table path only
auto-repairs product equations; coincidental ordinal/year additions remain
review-only.

The OCR evidence contract now exposes stable UI decisions:

- `AUTO_CONFIRMED`: the visual and structural evidence is accepted unchanged.
- `FORMULA_REPAIRED`: a value changed under a unique arithmetic proof.
- `MANUAL_REVIEW`: visual, structural, ontology or independent-verification
  authority is insufficient.

These statuses include success/info/danger presentation tokens for green,
blue and red review highlighting. Thousands separators are not inserted by a
global regular expression. A canonical display value is emitted only for a
column whose repeated-row contract proves grouped integers; the OCR source
token remains unchanged.

The long-term-repair ontology now accepts a separate field-alias data file.
The first data set contains 88 aliases across statutory work-item concepts,
including field terms such as `루프 방수`, `우레탄 도포`, `CCTV`, `부스터펌프`
and `물탱크`. Exact aliases resolve to a separate canonical semantic value;
unique fuzzy matches remain suggestions and are forced into manual review.
Neither path overwrites raw OCR text, and both report zero external OCR
invocations.

All 212 native OCR contract/regression tests pass. This hybrid layer improves
semantic recovery and reviewability but does not raise the raw visual character
metric by itself. The 99.7% promotion gate remains closed until new untouched
multi-family blind sets prove the full visual, structural and review pipeline.

## Sparse visual experts and deterministic table ranking (2026-09-30)

Decoder-width expansion is no longer an operating candidate.  Running a
512-path decoder on 143 routed rows consumed 163 seconds and removed only two
oracle edits.  The residual is therefore not primarily a search-width problem.
In contrast, a target-free view that removes only narrow rules spanning at
least 92% of a crop dimension added seven exact rows that the four stable
visual paths did not contain.  A second alternate view removes both full-height
vertical rules and full-width horizontal rules while preserving the original
pixels and short underlines.  At the trained 48-pixel geometry it reached
96.9672% greedy and 98.4808% 32-candidate oracle accuracy.  These transforms
remain candidate generators, not fact authorities.

The mixed-script router now wakes an expert from observable input only: low
confidence, structural-integrity failure, an ASCII run of at least three,
Hangul/Latin mixing, URL/e-mail/identifier punctuation, or symbol density over
20%.  Targets and document names are never read.  The recognizer trainer also
supports an L2 checkpoint anchor and a hard relative-drift projection.  This
allows future delta specialists to learn without silently moving the mature
visual basis.  Failed broad adaptations remain isolated research candidates.

A multi-source candidate ranker was added with three hard boundaries.  It can
select only B_Core-owned visual strings, it counts independent Top-3/Top-8
source agreement rather than mere deep-list occurrence, and it applies a
character-class mask only after a table compiler has issued a numeric,
cycle or work-item type.  Exact ontology surfaces may add evidence; fuzzy
ontology suggestions cannot auto-promote a string.  The first development run
improved generic character accuracy only marginally and still produced both
wins and regressions, so it is not promoted.  A target-free 64-candidate cap
also reduced oracle accuracy from 99.2294% to 99.0918%; the cap cannot be used
until ranking quality improves enough to retain rare correct candidates.

The arithmetic layer now implements the intended lazy CSP path.  It accepts at
most eight existing visual candidates per typed numeric cell.  When the visual
Top-1 tuple satisfies every repeated learned equation it exits immediately;
otherwise it searches the bounded Cartesian product and changes values only
when exactly one tuple satisfies all equations.  No number is generated by
language context.  On the already-seen Goldclass development table, 20 numeric
rows moved from 99.3450% character / 70% line exact at visual Top-1, to
99.8908% / 95% after same-fact typed formatting, and to 100% / 100% after one
unique arithmetic candidate repair.  This is proof that the deterministic
closed loop works, not an unseen-generalization claim.

The real page recognizer now carries each eligible table region's owned greedy
and CTC N-best strings into its geometry cell.  Numeric-column consensus
normalizes both the selected value and those owned candidates.  A mechanical
adapter then runs the same lazy arithmetic solver used by evaluation and copies
the selected value and proof receipt back into the cell before structured
evidence is compiled.  It infers no Korean header, apartment name, fixed column
or target label.  Multiline cells deliberately avoid a candidate cross-product
and remain conservative.

The table evaluator also no longer lets a fuzzy lexicon synthesize an unseen
replacement.  A lexical correction can become output only when the corrected
string is already present in the owned visual candidate set; otherwise it is
retained as a review suggestion.  This closed a concrete case where the visual
pool contained `년차 연도` but the old fuzzy pass rewrote another reading to
the unrelated `주차 용도`.

On the 604-cell independent-table regression corpus, the current v60/v61
research recognizer plus the closed table path measured 98.6646% character,
95.0331% cell exact and 99.3243% critical-token exact.  The older recorded v7
path on the same corpus was 96.7527%, 87.9139% and 98.9865%.  Because this
corpus has already been inspected in earlier development rounds, this is a
strong cross-document regression result but not a new untouched blind pass.
The corpus exposed 57 cells with more than one owned visual candidate; it had
no repeated product relation, so arithmetic CSP correctly made zero changes.

All 228 native OCR contract/regression tests pass.  No production checkpoint,
service route or external OCR dependency changed.  The 99.7% promotion gate
remains closed.  The next proof must use untouched long-term-repair tables with
typed cells and repeated equations, followed by five multi-family blind passes.

## Completion polish and B_Core runtime boundary (2026-09-30)

The repeated-equation path now discovers its arithmetic relation from the
visual N-best lattice rather than requiring correct Top-1 values first.  It
collapses candidates into bounded numeric sets, anchors agreed cells, and
materializes a change only when one distinct B_Core-owned visual tuple satisfies
the repeated relation.  Zero-solution rows remain review items; multiple
solutions remain ambiguous.  The solver never fabricates a missing number.

The production page path also gained sparse lazy expansion.  A wider CTC search
wakes only for numeric rows whose fast-pass candidate lattice is ambiguous or
unsatisfied.  An earlier retained Goldclass page 30 run woke nine rows and
added 177 owned visual candidates in 400.803 ms.  After the final candidate
lattice and punctuation pass, the same page no longer needed wide expansion:
the fast lattice closed every repeated relation in 11.578 seconds and emitted
11 `AUTO_CONFIRMED`, five `FORMULA_REPAIRED`, and four `MANUAL_REVIEW`
structured rows.  Target access and external OCR invocation were both zero.
This is an operational closed-loop check on retained development data, not a
blind 99.7% claim.

Surface punctuation cleanup is deliberately downstream of the numeric proof.
It removes a dangling period only when the parsed value is unchanged and the
independent table equation already holds.  Malformed alphanumeric candidates
cannot enter numeric columns, and stale punctuation review flags are cleared
only after the actual structural correction is present.

A stable process adapter now exports native OCR observations to B_Core.  Rows
carry page, box, confidence, decision status, fact authority, source and
evidence ID.  Review-only rows are emitted with non-authoritative confidence so
the semantic core cannot silently promote uncertain pixels into facts.  Model
weights remain separately versioned artifacts selected by explicit paths; the
source tree contains no implicit download and no external OCR fallback.

The final regression run passes all 233 native OCR contract tests on CPU.  A
parallel GPU run was not counted because another process exhausted device
memory during the suite.  Completion is therefore based on deterministic
contracts, reviewability and an auditable B_Core boundary, not on claiming the
unproven 99.7% visual metric.

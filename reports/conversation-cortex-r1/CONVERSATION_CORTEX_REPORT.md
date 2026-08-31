# Conversation Cortex R1

Status: `PASS_BOUNDED_CONVERSATIONAL_FRONTEND`

The first conversational layer is implemented in pure Rust. It preserves the raw utterance and keeps normalization, discourse signals, temporary references, canonical semantic concepts, and response realization in separate records.

Implemented behavior:

- Korean and English text input
- voice-transcript N-best candidates with confidence and fail-closed ambiguity handling
- inspectable typo and bounded fuzzy control-vocabulary normalization
- hesitation, floor holding, attention calls, acknowledgements, laughter, greetings, gratitude, and farewells
- explicit self-repair such as `파일을, 아니 폴더를 열어`
- onomatopoeia mapped to event properties rather than one semantic node per surface form
- turn-ordered, tamper-evident conversation state
- dynamic references such as `그걸` / `it`, with rejection when more than one referent is equally plausible
- natural conversational acknowledgement generated from the validated plan

Semantic separation remains intact:

- conversational semantic primitives: 13
- language-dependent semantic nodes: 0
- canonical semantic payload mutations: 0
- external LLM calls: 0
- network calls: 0

Validation:

- `cargo fmt --all -- --check`: pass
- workspace Clippy across all targets with warnings denied: pass
- semantic-core-adapters: 85 tests passed
- workspace: 546 tests passed, 0 failed
- `conversation-cortex-canary`: pass

This stage accepts an ASR transcript and alternatives; it does not yet decode raw audio or synthesize speech. It is a bounded conversational foundation, not a claim of GPT-level open-domain fluency. The next useful increments are a persistent conversation snapshot API, broader Korean morphology/spacing candidates, general entity grounding, and a dockable raw-audio ASR/TTS boundary.

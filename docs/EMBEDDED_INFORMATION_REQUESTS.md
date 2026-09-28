# Embedded information requests

This Rust engineering increment advances observation -> interpretation ->
attributed retrieval -> response. It does not promote semantic concepts, enable
recursive source application, or establish natural free conversation.

## Boundary change

The previous failing Korean request had an imperative COMMUNICATE frame, but
its theme was only `자세히`. The conversation contract simultaneously recorded
an information request and a separate task effect, preventing discourse QA.

ContentRequestIR now recognizes bounded why/how complements under a single
independent information/communication governor. Any additional recognized frames
must be content complements, not independent actions or conditions. It reads the
whole source argument, separates response manner, and rejects other recipients,
quoted inputs, negative requests and mixed action roots. A supported referential
event predicate binds to attributed discourse memory; explicit finite English
content remains an explicit source target. General Korean finite-clause joining
and other wh roles are not implemented by this change.

A validated content request may classify COMMUNICATE/INVESTIGATE as a response
effect, subject to recipient checks. A nonempty response-only effect set now also
establishes information_requested; classification and routing cannot disagree
on that obligation. This does not authorize sending a message to another person.

The pure manner grammar composes a head with bounded degree/softener tokens,
including comparative English detail and Korean degree/softener sequences.
These tokens do not become content targets. No new answer templates were added;
answers still use the existing source-bound projection and realization path.
Detailed manner does not yet produce richer explanatory derivations.

Direct why questions remain on their prior question/presupposition path unless
already supported independently. Embedded interpretation requires an outer
governor. This boundary was added after development tests caught overcapture of
ordinary why questions; all original regression tests pass on the frozen version.

## Evidence and limits

- Adapter library 694 + core library 29 = 723 passing tests.
- Six existing CLI regression suites, fmt, clippy and API build pass.
- Previous W evaluation, now development: 8/8 semantic requests met, formerly 6/8.
- New frozen X evaluation: 8 requests / 16 turns; 5 semantic full / 3 failed.
  All complete surfaces were absent from searched code, scripts and prior reports.
  This remains developer-authored/scored, not independently blind or GPT comparison.
- Original 50-turn and previous R outputs are unchanged.

Remaining failures are direct Korean anaphoric questions (`왜 그랬어?`), another
embedded wh role (`who wrote ...`), and recipient-aware third-party message
preparation. The latter still plans an explanation about the recipient rather
than a grounded message for them. Correct causal replies are still mechanical,
with quoted connective fragments and attribution disclaimers. No naturalness
completion claim follows from the semantic grades.

## Deployment

State49 / turn response40 / discourse answer4. Five Rust files and two docs are
mirrored into pakage; equality is not an independent package build. A new session
or explicitly authorized migration is required. No service restart, migration,
commit or push was performed. Canonical files, semantic generations, quarantine,
earlier failed reports and unrelated package edits remain intact.

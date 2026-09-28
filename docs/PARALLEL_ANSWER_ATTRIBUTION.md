# Parallel answer attribution scope

Multiple requested roles can be found correctly yet be spoken as disconnected
sentences with repeated attribution. This change coordinates existing answer
clauses during their initial morphological realization. It does not retrieve
finished sentences, rewrite generated text, or run another generation pass.

Only Inform clauses using the ordinary content-projection predicate qualify.
Both clauses must share a source speaker expression, the exact source belief
and event anchors, and an explicit semantic Sequence edge. Korean uses the
copular connective -이고 and a final register-sensitive ending. English uses
clause coordination. Subsequent attribution constituents receive a traceable
zero-width shared-scope grammar token; their meaning nodes are not deleted.
Different beliefs, events, speakers, speech acts or disconnected clauses do
not share attribution. Single-role, focused, plan and existing event-summary
paths retain their previous ownership.

Verification includes a native Korean multi-turn case with unchanged source
memory and one generation/one final check; formal and informal profiles;
English and Korean direct generation; five independently changed boundary
conditions; tampered-output rejection; and the unchanged prior regressions.

This is an improvement in clause composition, not proof of natural conversation.
Role labels and explicit source framing can still sound stiff. English compound
question parsing and conversational inference from a state to a suggestion
remain separate observed gaps. Do not use this result to claim those are fixed.

Schemas: generation 25, natural realization 38, conversation state 114,
conversation response 110. No state migration, deployment or service restart
is performed. No dictionary/canonical changes or autonomous semantic promotion.

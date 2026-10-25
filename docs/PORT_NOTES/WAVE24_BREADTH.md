# Wave 24: original command-owner invariants

The combined release checkpoint passes 2,020 tests (zero failures, one existing
ignore), m4 345 and m5 314; all-target clippy, 407 harness tests, schema checks,
five expected WASI guard traps, 6,003 native/WASM hash lines and web pass.
All nine worktree library data-path failures pass again in MAIN without optout.
Fresh 3,490-scenario results are byte-identical to wave 23: 3,354 exact,
115 not comparable, 21 existing failures, zero particle faults; startup retains
287 fixes. Both 610-game corpus tuples are unchanged (108/108 and 486/502).
Artifacts: `target/wave24/final-main/`; canonical source-keyed oracles are
`target/sweeps/f570da10b93561b33a35501bc0e6c9ac01d81562.*`.

Quiet performance passes timing (18.552 ms/600 ticks, 56.702 ms load).
Unchanged sizes (7,675,616 stripped, 6,897,664 text) and Kirby's one duplicate
label retain existing budget failures. Shared duplication passes (20/94).
No threshold was raised; `docs/PERF.md` retains the complete report.

Production source is f570da10b93561b33a35501bc0e6c9ac01d81562
(tree a21a201bfc9734964d51fced2885d4bbb49de923). The two scoped local picks
are bbfded5f (Swing42) and f570da10 (Smash42), reviewed from c2acc278 and
bed1cd1d respectively. No new gameplay, item, character, mode or callback is
admitted. Both missing static capabilities continue to panic, matching the
retail invariant rather than selecting a fallback motion.

Swing42's proof covers all 33 ordinary command owners, 25 separately
identified Kirby copy-data roots, all six item classes and 792 common swing
scripts (744 nonnull). Exactly six Captain smash scripts write the flag.
Entry clears the whole throw word and replaces borrowed script ownership;
demo fighters retain their actual scheduler but have no normal input proc.
The outgoing-hook audit closes all 31 installations / 18 qualified targets.
The actual Captain 520-tick recording includes one authored transition and
45 private-row ticks; the Peach 1,400-tick control remains ordinary swings.

Smash42's proof covers all 33 ordinary owners and 165 common S4 scripts
(135 nonnull). Link, Young Link and Ness are the three real variable writers;
Ness consumes its variable through a distinct private bat callback. The
source census closes 1,132 C calls plus three executable header calls,
including all literal and dynamic selectors. The 365-candidate writer audit
also closes asynchronous mutations while common S4 owns its callbacks.
The live test retains all 6,360 ticks across Link110, YoungLink5330,
Fox300 and Ness620. Link proves entry only. Young Link proves 39 private
rows and the recorded exit; Ness uses its actual private callback.

Stock Link/YoungLink scripts clear the combo variable immediately before
enabling interrupts. This proves their windows disjoint under the certified
owners. The existing structural input-priority order is unchanged; this is
not a claim of complete IASA parity or arbitrary custom-script behavior.

All new raw comparisons obtain named fields from the generated schema and
read them through hsd-archive. The long Stadium fixture supplies its recorded
external read-completion input at tick 3757, alongside original controller,
queue and display inputs. No expected game-state field is used as an input.

Detailed proofs: SWING42_COMMAND_OWNER_INVARIANT.md and
SMASH42_COMMAND_OWNER_INVARIANT.md. Original captures, scripts, inputs and
expectations are unchanged. No recordings, protected-data copies/symlinks,
decomp edits, pushes or threshold increases are part of this wave.

The user authorized new captures and required boundaries on 2026-10-25,
superseding the prior recording restriction. Ness's original 4,775-tick scene
has only 3,340 particle/meta ticks; a fresh full capture is first in the quiet
queue. Preserve old recordings and failed attempts, use fresh prefixes and
never commit game data. Heavy carry/throws, held-item and monster callbacks,
copied Sheik and Flat Zone evidence are next. The bounded third-invariant
search found no eligible classification among its inspected candidates; this
is not an impossibility claim. Every retail reporting group remains in scope.

> **Branch** `worktree-agent-a01ba14eedf3e5707` (tip `b3b9e44e`), not merged into main. Based on 26fee4c7. Verified: 6099e03c, a01f4737, f6cd6bee (full sweep 0 regressions, 2 fixed); 37da877a gates exact but has had no full sweep. **97711e50 is WIP** (unverified Stone phantom hit; drop it or restore the unimplemented! before merging). To merge, rebase onto main (first conflict: crates/melee-ft/src/fighter/assets.rs), sweep, replay-batch.

# Handoff: wave 13, PARITY items 19-21 (grab/throw/capture edges, grab pairs in status effects, common-state edges)

Branch `worktree-agent-a01ba14eedf3e5707`, based on main 26fee4c7.

## Task and scope

`docs/PARITY.md` work items 19 (14 grab/throw/capture sites), 20 (3 grab
pairs inside status effects) and 21 (15 common-state edges). The full site
list (file:line, retail function, message) is in
`/Users/nikhilunni/Projects/melee/target/parity/fail_closed.tsv`; filter it
by the files that PARITY.md names for these items. The goal: close each
`unimplemented!` with a retail witness that gates exact, or write down
evidence that retail cannot reach it.

## Done and verified (commits)

- `6099e03c` melee-ft: six sites closed by the retail call graph. Evidence is
  in `docs/COVERAGE_AUDIT.md` ("Investigations resolved"):
  - `state/callbacks/input.rs` FallSpecial IASA: ported whole (parasol, catch
    window, then the aerial jump via `try_aerial_jump`). Special fall with
    jumps left is unreachable, because every inline0 entry spends them.
  - `jump.rs` enter_knee_bend: the None arm is now `unreachable!`, since every
    caller tests `human::jump_input` first.
  - The aerial catch-alls in `input.rs` and `damage.rs` (Fall and Damage
    IASA) are now `unreachable!`: `fall::iasa_with_jump` yields only the
    handled arms.
  - `air_dodge.rs` tether: when the hook is absent the function now returns
    false. The duplicated `air_dodge_tether` descriptor flag is removed.
  - `grab.rs` fn_800DB6C8: retail switches on the captor's tether, not the
    victim's, and the victim's tether went with ftCommon_8007DB58. This is now
    an assert.
- `a01f4737` ft-koopa: the Klaw holder losing its floor
  (ftKp_SpecialS_801332C4 / 80133398 / 80133484), as `LostHold::Klaw` in
  `grab_damage.rs`. The witnesses are in MATRIX_WITNESSES, exact over 5560
  ticks with 0 differing particle-site ticks:
  - `koopa_klaw_windmill_wall_ps_bowser_ness4` (Wait)
  - `koopa_klaw_throw_held_windmill_wall_ps_bowser_ness4` (EndF before the release)
  - `koopa_klaw_throw_windmill_wall_ps_bowser_ness4` (after the release)

  Their particle dumps timed out while the machine was under load, so the
  sweep reports NOTCMP; re-recording them on a quiet machine would fix that.
  The Hit/Bite rows (801332C4) share the same code but were not reached: a
  `melee-sim search` over 13,923 candidates found none.
- `f6cd6bee` COVERAGE_AUDIT gains two rows:
  - Item 20 (`damage_song.rs:25`, `damage_bind.rs:54`, `capture_mewtwo.rs:41`)
    needs a third fighter.
  - The Egg Lay and Kirby mouth release of an invincible fighter needs a
    Starman.

  Both stay `unimplemented!`. This commit also carries the MATRIX_WITNESSES
  count fix (563) from the rebase.
- `37da877a` down.rs: ftCo_DownDamage_Coll's grounded arm now runs
  ftCo_800C7CA0 (`try_down_reflect`). The last-wall byte is read from the top
  byte of `mv.co.damage.x4`. The bounce out of DownDamage (fn_800C7DC4 with
  Damage scratch) is still unwitnessed and fails closed in
  `enter_down_reflect`.
  - Witness: `corpus_v3_ps_bowser_ness4_e317915a4_p1` in CORPUS_V3_MATCHES,
    exact over 4900 ticks with 0 differing particle-site ticks. Bowser hugs a
    wall in DownDamageD at 4771 and does not bounce.
  - **No full sweep was run after this commit** (I was stopped mid-sweep).
    The sweep of the three commits before it, against 26fee4c7.tsv, showed
    0 regressions and 2 fixed.

## WIP commit

- `97711e50` WIP: `hit_log.rs` expire_phantom_lockout against the x221C_b4
  shell (ftColl_8007BE3C, ftcoll.c:3653-3676), plus the scenario
  `kirby_stone_phantom_fd_fox4`, which is recorded.
  - The port is unverified. The scenario diverges at 277 (`p0.cur_pos.z`)
    before reaching the site: the port treats Fox's down-angled forward tilt
    (272) on Stone Kirby as a phantom and gives Kirby 3 ticks of hitlag. On
    retail Kirby's depth recentering keeps moving through 277-281, so retail
    gave no hitlag; most likely it classified the contact as an ordinary hit
    that the shell absorbed.
  - The suspect is coll_distance at shallow overlap with nonzero z (Kirby
    -0.7, Fox +0.7) in `melee-coll` capsule_contact vs lbColl_80006E58.
    This is a real port divergence outside the item scope.
  - Before merging, either drop this commit or revert `hit_log.rs` to its
    `unimplemented!`, and fix the classification first.

## Where I stopped / next steps

1. Run the full sweep against `/Users/nikhilunni/Projects/melee/target/sweeps/26fee4c7.tsv`
   (or a newer baseline) on the non-WIP commits. The commit 37da877a still
   needs one.
2. Investigate the phantom classification (`kirby_stone_phantom_fd_fox4`
   triage, tick 272-277). Compare lbColl_80006E58's asm with
   `melee_coll::geometry::capsule_contact` for z handling.
3. DownReflect bounce from DownDamage: bridge the third explorer case
   (`v3-ps_bowser_ness4-explore3e075aba-profile0`, ticks=5692) or search on
   `corpus_v3_ps_bowser_ness4_e317915a4_p1` for a bounce. Then port the
   scratch carry in `enter_down_reflect`: mv+0 = hitstun, mv+4 = mv_word with
   the side in the top byte.
4. Klaw Hit row: search with Ness lower or Bowser riding a later blade; the
   wall events come about 180 ticks apart, at 5260, 5440 and so on.
5. Remaining open sites and why:
   - `walk.rs:235` walk tier: the rate is the caller's f31, which
     Fighter_8006A360 never sets; it comes from the GObj proc loop.
   - `life.rs:352` frozen top KO and `damage.rs:1414` freeze by throw: need an
     Ice freeze at high percent.
   - `damage.rs:1125`: cape on a frozen fighter. Needs a third fighter or a
     Freezie.
   - `bury.rs:251`: mound on a moving floor. Needs a bury move (DK side-B)
     on a moving platform, plus the mound translate effect.
   - `collision.rs:462`: revival platform.
   - `collision.rs:526`: dash throw off a ledge. Needs items and a windmill.
   - `throw_flight.rs`: landings before the flag. Try Kirby's up throw under
     a Yoshi's Story platform.
   - The other `grab_damage`/`grab_escape`/`capture_yoshi`/`cargo` sites:
     not attempted.

## Commands

```sh
CARGO_INCREMENTAL=0 cargo build --release -p melee-sim
./target/release/melee-sim gate /Users/nikhilunni/Projects/melee/harness/scenarios/<name>.toml
./target/release/melee-sim particle-sites --to <frames-1> /Users/nikhilunni/Projects/melee/harness/scenarios/<name>.toml
cd /Users/nikhilunni/Projects/melee/harness && uv run python record_many.py scenarios/<name>.toml --jobs 4
```

The Klaw scenarios were built with `melee-sim search` from base
`corpus_v3_ps_bowser_ness4_e404fe6bc_p0`, truncated at 4950. Its water form is
created at tick 4240, the same tick as in the Mewtwo windmill game.

## Recordings and boundaries

No new boundaries. New retail traces are in the main checkout's
`harness/traces`:

- the three `koopa_klaw_*` scenarios
- `kirby_stone_phantom_fd_fox4`
- `corpus_v3_ps_bowser_ness4_e317915a4_p1` (frames extended to 4900)
- `corpus_v3_ps_bowser_ness4_edb5ae9dd_p0`

The scenario TOMLs are in the main checkout's `harness/scenarios`. Explorer
recordings are in this worktree's `target/w13/explore/rec_bowser`.

## Gotchas

- `melee-sim search` reach conditions are ordered and can be satisfied on the
  same tick. An avoid condition also applies to the shared prefix.
- A binary built while hack prints were in the source keeps them until the
  touched files rebuild. Check with `strings`.
- Particle dumps time out (600 s) when the machine is loaded.
- Explorer faults on Stadium do not always reproduce once bridged, because
  retail reads the Stadium archive at different times.

## Divergences outside my area

- `corpus_v3_ps_bowser_ness4_edb5ae9dd_p0` diverges at 924: retail creates
  Ness's item kind 102 and the port does not.
- `kirby_stone_phantom_fd_fox4`: phantom vs ordinary-hit classification (see
  the WIP section).
- Explorer faults from `start_ys_kirby_fox4`:
  - ftCo_800BDA74 (star landing on a floor) ×5
  - fn_800F6AC8 (the gulp leaving the floor)
  - ftKb_SpecialLwEnd_Coll (end row touching a floor)
  - it_2725_Logic7_ShieldBounced (Final Cutter beam)

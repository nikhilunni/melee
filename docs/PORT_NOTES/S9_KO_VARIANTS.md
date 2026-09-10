# S9: high-percent KOs: damage sounds, fly roll, side and top blast zones

2026-09-10, combat lane, ported directly (Codex credits were exhausted at the
time). **Five of the seven S9 scenes are exact; `topko2_usmash_fd_fox` stops
explicitly at the camera-space screen-KO approach; the two `hi200_uthrow`
scenes stop at ThrowHi (S7).** No scenario, trace, decomp or expected value was
changed.

## Scene evidence

| Scene | Recorded reaction states (tick:state) | First-divergence history | Final release CLI |
|---|---|---|---|
| `hi200_jab_fd_marth` | P2 DamageN3 80 -> Fall -> off the edge -> DeadDown 0 -> Rebirth | heavy voice queue -> exact | 300 ticks, 62 keys, 0 divergences |
| `hi200_utilt_fd_marth` | P2 285:87 -> 324:2 (DeadRight, x=249.8) -> 384:12 -> 444:13 | `life.rs` side/up death stop -> exact | 420 ticks, 62 keys, 0 divergences |
| `hi200_dolphinslash_fd_marth` | P2 282:88 -> 323:2 -> 383:12 -> 443:13; P1 367 -> 35 -> 43 | same -> exact | 420 ticks, 62 keys, 0 divergences |
| `topko_usmash_fd_fox` | P2 267:90 -> 293:4 (DeadUpStar at y=191.2), flying from 294 to the end | fly-roll draw -> death stop -> exact | 230 ticks, 62 keys, 0 divergences |
| `topko2_usmash_fd_fox` | P2 271:90 -> 297:6 (DeadUpFall) -> 348:7 (HitCamera) | death stop -> explicit stop at tick 298 (`ftDrawCommon_80080E18_inline2`) | STOP: entry tick exact, approach needs the camera port |
| `hi200_uthrow_fd_marth`, `hi200_uthrow2_fd_marth` | ThrowHi 221 / ThrownHi 241 -> DamageFlyTop 90 -> tumble -> DownBoundD | S7 ThrowHi (140 / 146 ticks matched) | BLOCKED on S7 |

Regressions kept exact: `ko_fd_marth` (480), `usmash_fd_fox`, `jab_fd_marth`,
`match_fd_marth_scripted` (1,600). The three exact new scenes are in
`m5_gate.rs` and `alloc_gate.rs` (zero simulate-only allocations).

## What was ported

### Damage sounds and the fly roll (`damage.rs`, `assets.rs`)

- `ftCo_8008DCE0`: when the scaled knockback (`kb * PlCo+154`, the port's
  `stun`) reaches PlCo `+20C` the heavy hit sfx 0x4F and a voice from
  `x4C_sfx->x20` are queued; at `+208` the medium pair (0x50, `x4C_sfx->x1C`).
  `ftCo_80090718` plays them the next frame from `Fighter_ProcessHit`'s
  no-new-hitlag branch: `ft_PlaySFX(id, 127, 64)` and `ft_800889F4` ->
  `HSD_Randi(num)` over the voice table (ledger site 0x80088a18).
- DamageFlyRoll: level 3, airborne after the launch (or launched upward),
  not the top angle, percent with the hit >= PlCo `+23C` (int) and
  `HSD_Randf() < +240` (site 0x8008e124).
- `proc_process_hit` now takes the RNG; the three fixture harnesses pass it.

### Blast-zone dispatch (`life.rs::check_blast_zone`, `ftCo_800D3158`)

Retail order: right, left, top, bottom. A top exit only counts when the
fighter is grounded, carries `x2222_b3` (DamageIce / ShieldBreakFly entries;
the ShieldBreakFly case is an explicit stop until S6 merges) or has
`kb_vel.y > PlCo+4F0` (2.4). Then `HSD_Randi(100) + 1` (site 0x800d326c):
`PlCo+520 (16) >= roll` is the screen KO (`ftCo_800D4780`), otherwise the
star KO (`ftCo_800D40B8`). `Player_GetMoreFlagsBit5` (plain DeadUp 3) and
`Camera_8003010C` (fixed camera) are off in a Vs match.

### The shared death entry (`ftCo_800D331C` + the per-direction tails)

`release_for_death`: linked-fighter release (still an explicit stop),
`ftCommon_8007E2FC` velocity clear (`clear_velocities`), `ftCommon_8007DB24`
(`x2219_b0 = 0` then `efLib_DestroyAll`, pushed as `EffectRequest::DestroyOwned`).
Held items, metal, the x2226_b4 hat and the death1/2/3 hooks are not in the
port yet (Fox and Marth have no hooks). Then per direction:

| Entry | State | Exit sfx | Explosion (efSync 0x42B) position / rotation Z | Stock loss |
|---|---|---|---|---|
| `ftCo_800D3BC8` | DeadDown 0 | 0x61 | x clamped to the side blast zones, 0 | at entry |
| `ftCo_800D3680` | DeadLeft 1 | 0x88 | y clamped to top/bottom, -pi/2 | at entry |
| `ftCo_800D3950` | DeadRight 2 | 0x89 | y clamped to top/bottom, +pi/2 | at entry |
| `ftCo_800D40B8` | DeadUpStar 4 (anim DamageFall 29) | voice `x4C_sfx->xC` | none at entry | at the vanish |
| `ftCo_800D4580(6)` | DeadUpFall 6 (anim DamageFall 29) | none | none at entry | at phase 3 |

`EffectRequest::Death` now carries the rotation-Z argument (efasync.c:598 sets
rotation, then the uniform scale PlCo `+4F4`). The dead rows with
`ftCo_SM_None` enter at animation frame -1 (`spawn.rs`), as the traces show.
Every entry also plays `x4C_sfx->x4` and `->x8` on the voice channel
(`ftCo_800D38B8`), stamps the killer's stale-move table (`pl_8003DF44`, no
compared key) and requests `Camera_RequestQuake(Large)` plus rumble, which
have no simulated observer.

The stock-loss bookkeeping `ftCo_800D34E0` (falls, KO/suicide counts, HP=0) is
what fires the HUD percent explosion (`Player_UpdateKOsBySlot` ->
`ifStatus_TriggerStockLoss`): the sim's `set_dead` therefore keys on
`LifeState::Dead`, which a star KO only enters when it vanishes.

### Star KO (`ftCo_DeadUpStar_Anim`, 800D42E4)

Scratch `LifeState::StarKo { remaining, flying, camera_top }`. Hold PlCo
`+504` (1) frame, then, with `flight = +508` (130):

```
self_vel.y = fmsubs(+514 (0.6), Stage_GetCamBoundsTopOffset(), cur_pos.y) / (f32) flight   // 800D4438 fmsubs, fdivs
self_vel.z = +510 (-350.0) / (f32) flight
```

No retail physics callback: `Fighter_procUpdate`'s tail integrates the
velocity (`free_flight_physics`, now shared with the revival platform).
The trace covers 90 of the 130 flight frames; the vanish (velocity clear,
`efAsync_Spawn 0x42D` twinkle, sfx 0x83, stock loss, PlCo `+50C` = 45 frames
to the respawn request) is an explicit stop until a 400-frame recording of
`topko_usmash_fd_fox` exists.

### Screen KO (`ftCo_800D4580`, `ftCo_DeadUpFall_Anim/_Phys`)

Only the entry is ported (`LifeState::ScreenKo { remaining }`, PlCo `+524` = 1
frame hold, nametag countdown, colour animation 0x2B). From the next frame
retail sets `x2220_b7` and the render callback `ftDrawCommon_80080E18_inline2`
writes `cur_pos = InvViewingMtx(camera) * mv.x50`, where `x50` is lerped from
PlCo `+538` to `+544` over `+528` (50) frames, then `DeadUpFallHitCamera 7`
(`+52C` = 3 frames; `ft_800889F4` heavy voice draw), then the fall (`+550` /
`+554` / `+558` / `+55C` = 1.0, 0.2, 1.7, -1.0 for `+530` = 40 frames) and the
vanish (`+534` = 35). The recorded positions from tick 298 on are camera
space, so the whole approach is blocked on the camera port; the port stops at
the phase-0 -> 1 transition with the retail function named.

## Stops left explicit

- `ftCo_DeadUpStar_Anim` vanish (needs the longer recording).
- `ftDrawCommon_80080E18_inline2` screen-KO approach (camera port).
- `ftCo_ShieldBreakFly.c:30` top-exit flag (S6), DeadUp 3 (special modes),
  DamageIce variants, linked-fighter release on death (S7).

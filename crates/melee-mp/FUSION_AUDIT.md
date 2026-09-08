# melee-mp retail fusion audit

Audited against NTSC-U 1.02 `main.dol`, using `harness/asm.py` and the
decomp-toolkit split of `melee/mp/mplib.c` and `melee/mp/mpcoll.c`.
The 39 original markers represent 38 arithmetic sites and one crate-doc
statement: 33 arithmetic sites are fused and five are unfused. All are
resolved. Two additional unmarked arithmetic sites were corrected nearby.

The site labels below use the **pre-audit** source line numbers so they
remain identifiable against the original request. Addresses are hexadecimal;
slash-separated suffixes reuse the preceding address prefix. Ranges with a
stride list repeated instructions, not every instruction in the range.
The source comments give the same evidence beside each expression.

| Site | Rust function / expression | C function | Retail address(es) | Instruction sequence | Fused? |
|---|---|---|---|---|---|
| geom:48 | `remap_2d`: squared length | `mpRemap2d` | `0x8004DCA0`, `0x8004DCB0` | `fmul`, `fmadd` (double) | Yes |
| geom:52 | `remap_2d`: projection numerator | `mpRemap2d` | `0x8004DCE8`, `0x8004DCF0` | `fmul`, `fmadd`, `fdiv` | Yes |
| geom:59 | `remap_2d`: output x/y | `mpRemap2d` | `0x8004DD30/34/38/3C` | Two `fmadd` per coordinate, then `frsp` | Yes |
| geom:112 | `line_intersection`: start half-space | `mpLineIntersection` | `0x8004EA60/64` | `fmul`, `fmsub` | Yes |
| geom:125 | `line_intersection`: end half-space | `mpLineIntersection` | `0x8004EA94/98` | `fmul`, `fmsub` | Yes |
| geom:139 | `line_intersection`: determinant | `mpLineIntersection` | `0x8004EAD8/DC` | `fmul`, `fmsub` | Yes |
| geom:152 | `line_intersection`: area | `mpLineIntersection` | `0x8004EB50/58` | `fmul`, `fmsub` | Yes |
| geom:158 | `line_intersection`: barycentric numerator | `mpLineIntersection` | `0x8004EB80/88` | `fmul`, `fmsub`, `fdiv` | Yes |
| geom:162 | `line_intersection`: output x/y | `mpLineIntersection` | `0x8004EBA4/A8` | `fmadd`, then `frsp` per coordinate | Yes |
| geom:218 | `line_intersection_h`: x | `mpLineIntersectionH` | `0x8004ECE8/F4` | `fdiv`, `fmadd`; `frsp` at `0x8004ED48` | Yes |
| geom:275 | `line_intersection_v`: y | `mpLineIntersectionV` | `0x80050158/64` | `fdiv`, `fmadd`; `frsp` at `0x800501B8` | Yes |
| map:604 | `update_joint_transform`: vertex x | `mpLib_80055E9C` | `0x800560F0`–`0x80056198`, stride `0x18`; `0x800561C4` | `fmadds` (eight-way unroll and tail) | Yes |
| map:606 | `update_joint_transform`: vertex y | `mpLib_80055E9C` | `0x800560FC`–`0x800561A4`, stride `0x18`; `0x800561D0` | `fmadds` (eight-way unroll and tail) | Yes |
| map:610 | `update_joint_transform`: four bounds | `mpLib_80055E9C` | `0x800561EC`, `0x80056200/14/28` | `fmadds`, then separate `fsubs`/`fadds` for 30-unit padding | Yes |
| query:213 | `floor_probe`: height | `mpLib_8004DD90_Floor` | `0x8004E02C/30/34/38/3C/40` | `fmuls`, `fdivs`, `fadds`, `fsubs`, double `fadd`, `frsp` | No |
| query:274 | `ceiling_probe`: height | `mpLib_8004E090_Ceiling` | `0x8004E334/38/3C/40/44/48` | `fmuls`, `fdivs`, `fadds`, `fsubs`, double `fsub`, `frsp` | No |
| query:333 | `left_wall_probe`: x delta | `mpLib_8004E398_LeftWall` | `0x8004E628/2C/30/34` | `fmuls`, `fdivs`, `fadds`, `fsubs` | No |
| query:392 | `right_wall_probe`: x delta | `mpLib_8004E684_RightWall` | `0x8004E920/24/28/2C` | `fmuls`, `fdivs`, `fadds`, `fsubs` | No |
| query:539 | `sweep`: direction dot product | `mpCheckFloorRemap`; `mpCheckCeilingRemap`; `mpCheckLeftWallRemap`; `mpCheckRightWallRemap` | `0x8004F688/F77C`; `0x8004FE54/FF54`; `0x800507A0/0894`; `0x80050F8C/1080` | Rounded y product, then `fmadds` with x product; each general/axis-aligned branch checked | Yes |
| query:839 | `wall_vertex_sweep`: direction dot product | `mpLib_800511A4_RightWall`; `mpLib_800515A0_LeftWall` | `0x800513B0/14B0`; `0x800517AC/18AC` | Rounded y product, then `fmadds` with x product; both vertices | Yes |
| query:958 | `floor_below`: projected height | `mpLib_8005199C_Floor` | `0x80051B18/1C` | `fdivs`, `fmadds` | Yes |
| query:1342 | `walk_along_floor`: output x/y/z | `mpLib_80056C54` | `0x800573A0/B4/C8` | `fmadds` per component | Yes |
| mpcoll:367 | `load_ecb_jobj`: side midpoint y | `mpColl_LoadECB_JObj` | `0x800428E4/E8/F8` | `fmuls`, two `fadds` reusing the rounded half-sum | No |
| mpcoll:427 | `load_ecb_fixed`: rotate right point | `mpColl_LoadECB_Fixed` | `0x80042AEC/F0/F4/F8` | Two `fmuls`, then `fmsubs` for x and `fmadds` for y | Yes |
| mpcoll:433 | `load_ecb_fixed`: rotate left point | `mpColl_LoadECB_Fixed` | `0x80042B34/38/3C/40` | Two `fmuls`, then `fmsubs`, `fmadds` | Yes |
| mpcoll:523 | `vec2_interpolate`: x/y | `Vec2_Interpolate`, inlined into `mpCollInterpolateECB` | `0x80042E68`–`0x80042EF4`, stride `0x14` | Eight `fmadds`, one per ECB component | Yes |
| mpcoll:1133 | `ceiling_left_wall_multi_collide`: endpoint | `mpColl_800439FC` | `0x80043A4C/50` | `fmadds`, `fnmsubs` | Yes |
| mpcoll:1156 | `ceiling_right_wall_multi_collide`: endpoint | `mpColl_80043ADC` | `0x80043B2C/30` | `fnmsubs`, `fmadds` | Yes |
| mpcoll:1203 | `floor_left_wall_multi_collide`: floor endpoint | `mpColl_80043C6C` | `0x80043D04/10` | `fnmsubs`, `fmadds` | Yes |
| mpcoll:1287 | `floor_right_wall_multi_collide`: floor endpoint | `mpColl_80043F40` | `0x80043FD8/E4` | `fmadds`, `fnmsubs` | Yes |
| mpcoll:1316 | `floor_right_wall_multi_collide`: corner extension | `mpColl_80043F40` | `0x80044098/B0` | `fmadds`, `fnmsubs` | Yes |
| mpcoll:2214 | `wall_collide`: right ceiling corner | `mpColl_800454A4_RightWall` | `0x80045820/2C/34` | `fneg` of normal y, `fdivs`, `fmadds` | Yes |
| mpcoll:2247 | `wall_collide`: left ceiling corner | `mpColl_80046224_LeftWall` | `0x80046598/B0/B4` | `fneg` of normal x, `fdivs`, `fmadds` | Yes |
| mpcoll:2297 | `wall_collide`: first traversal, bottom edge | Wall quartet below, in order | `0x800458F8`, `0x80046688`, `0x80049500`, `0x8004A1E4` | `fsubs`, `fmadds` | Yes |
| mpcoll:2300 | `wall_collide`: first traversal, top edge | Wall quartet below, in order | `0x80045924`, `0x800466B4`, `0x8004952C`, `0x8004A210` | `fsubs`, `fmadds` | Yes |
| mpcoll:2331 | `wall_collide`: second traversal, bottom edge | Wall quartet below, in order | `0x800459FC`, `0x8004678C`, `0x80049604`, `0x8004A2E8` | `fsubs`, `fmadds` | Yes |
| mpcoll:2334 | `wall_collide`: second traversal, top edge | Wall quartet below, in order | `0x80045A28`, `0x800467B8`, `0x80049630`, `0x8004A314` | `fsubs`, `fmadds` | Yes |
| mpcoll:2889 | `find_new_floor_below`: previous midpoint | `mpColl_8004A908_Floor` | `0x8004AA5C/6C` | `fadds`, `fmadds` | Yes |
| lib:15 | Crate documentation | N/A | N/A | Replaced obsolete audit-status statement with reference to this report | N/A |

The wall quartet is `mpColl_800454A4_RightWall` (air right),
`mpColl_80046224_LeftWall` (air left), `mpColl_800491C8_RightWall` (ground
right), `mpColl_80049EAC_LeftWall` (ground left). Every instance was checked;
their shared Rust expressions have the same fusion despite register changes.

Additional findings corrected:

| Rust function / expression | C function | Retail address(es) | Instruction sequence | Fused? |
|---|---|---|---|---|
| `wall_vertex_sweep`: motion-length threshold | `mpLib_800511A4_RightWall`; `mpLib_800515A0_LeftWall` | `0x80051358/5C`, `0x8005145C`, `0x80051758`, `0x80051858` | y-square `fmuls`, then x-square `fmadds` | Yes |
| `floor_left_wall_multi_collide`: mirrored corner extension | `mpColl_80043C6C` | `0x80043DC4/DC` | Two `fnmsubs` | Yes |

## Oracle and observations

`tests/geom_oracle.rs` compiles verbatim and retail-faithful C extracts with
`-O0 -ffp-contract=off -fno-builtin -fno-strict-aliasing -fwrapv`.
The retail copy uses the shared `gekko_fma.h`. The source-extract test checks
the verbatim functions against the submodule, independently of the Rust.
Each output record contains the hit flag and both coordinate bit patterns.
Miss records use zero coordinates. There is no epsilon or NaN exception.

| C oracle | Inputs | Successful outputs | Rust/retail bit mismatches | Input records changed by fusion |
|---|---:|---:|---:|---:|
| `mpRemap2d` | 50,000 | 50,000 | 0 | 29 |
| `mpLineIntersection` | 50,000 | 9,408 | 0 | 4,279 |
| `mpLineIntersectionH` | 50,000 | 9,841 | 0 | 25 |
| `mpLineIntersectionV` | 50,000 | 2,652 | 0 | 21 |

The finite sweeps include game coordinates, independently varying exponents,
signed zeros, degenerate and collinear lines, endpoints, the 1e-4 threshold,
and cancellation. Counts compare entire input records, not individual lanes.
In particular, general intersections near the origin can retain a tiny
double-FMA residual through the final `frsp`, whereas separate multiplication
and addition produce zero. Double precision does not make fusion irrelevant.
The original behavioral-test expected values were unchanged.

No paired-single instruction occurs directly in either mp translation unit.
The delegated `PSMTXMultVec` uses `ps_mul` at `0x80342AB4`, lane-wise
`ps_madd` at `0x80342ABC`, then `ps_sum0` at `0x80342AC4` (repeated for
the other rows). Its lanes compute `m2*z + rounded(m0*x)` and
`m3*1 + rounded(m1*y)`, then add separately. The existing Rust helper
preserves that structure. `PSVECNormalize` likewise uses `ps_mul` at
`0x80342DC4`, `ps_madd` at `0x80342DCC`, and `ps_sum0` at `0x80342DD0`:
the length is `fma(z,z,rounded(x*x)) + rounded(y*y)`. Its final `ps_muls0`
broadcasts the low-lane reciprocal length. Neither helper was modified.

The motion-length threshold is fused even though the adjacent squared hit
distance is **unfused** (`0x800513A8/AC/B4`: two `fmuls`, then `fadds`).
Those expressions must not share one indiscriminately fused distance helper.
The four surface probes also stay unfused: their division separates the
multiply from the add. Negated fused forms use `fnmsubs`, including its
round-then-negate zero-sign behavior.

The native oracle covers every marked `geom.rs` expression. `sq` and the
tolerance predicate have no multiply-add; normalization belongs to the
delegated HSD helper. Stateful map/query/collision changes use retail
instruction evidence and existing behavioral/real-stage tests, rather than
a new stateful C oracle. No Dolphin execution comparison was added.

## Files and validation

Changed: `src/geom.rs`, `src/lib.rs`, `src/map.rs`, `src/mpcoll.rs`,
`src/query.rs`. Added: this report, `tests/geom_oracle.rs`,
`tests/ref/mplib/geom.c`, `tests/ref/mplib/retail/geom.c`,
`tests/ref/mplib/driver.c`, `tests/ref/mplib/NOTICE`.

Validation completed:

- No unresolved fusion markers remain under `crates/melee-mp` (recursive search).
- `cargo test -p melee-mp -- --nocapture`: 55 tests passed, including all
  200,000 native-C input comparisons and three real-stage tests.
- `cargo clippy -p melee-mp --all-targets -- -D warnings`: passed.
- `cargo test --workspace --exclude hsd-anim`: passed, both before and
  after the audit. HSD still builds as a dependency; its own tests are excluded.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `git diff --check -- crates/melee-mp`: passed.

No commits, game-data additions, or edits outside `crates/melee-mp` were made.

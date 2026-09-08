# hsd-anim retail fusion audit (NTSC-U 1.02)

The 31 original marker lines comprise 26 arithmetic groups and five overview
notes. Twenty arithmetic groups contain fused instructions; six groups are
unfused scalar sums of squares. All addresses below were read from the retail
DOL through `harness/asm.py` and the existing dtk assembly split. No SDK
paired-single kernel contained an unresolved arithmetic marker.

## Every original marker

Ranges list the first and last relevant instruction, not a claim that every
instruction between them is fused. Multiple expressions covered by one original
marker remain together in this table. File line numbers identify the original
marker in commit 96fa969, so the inventory survives edits to the Rust.

| Original site | Rust function / expression | C function | Retail address(es) | Instruction(s) | Fused? |
|---|---|---|---|---|---|
| mtx.rs:32 | Module overview | HSD matrix routines | See rows below | Mixed scalar arithmetic; SDK PS helpers already explicit | Mixed; note updated |
| mtx.rs:556 | `hsd_calc_determinant_3x4` | `HSD_CalcDeterminantMatrix3x4`, inline in all three inverse functions | 0x80379368–0x80379384; 0x803795F4–0x80379614; 0x80379A70–0x80379A8C | 2 `fmadds`, 3 `fnmsubs` per inline expansion | Yes |
| mtx.rs:584 | `hsd_mtx_inverse`, nine cofactors | `HSD_MtxInverse` | 0x803793EC, 0x8037940C, 0x8037942C, 0x8037944C, 0x8037946C, 0x8037948C, 0x803794AC, 0x803794CC, 0x803794EC | Alternating `fmsubs` / `fnmsubs`, then separate `fmuls` | Yes |
| mtx.rs:595 | `hsd_mtx_inverse`, translation | `HSD_MtxInverse` | 0x80379518/1C, 0x80379544/48, 0x80379570/74 | `fmsubs` + `fnmsubs` per row | Yes |
| mtx.rs:621 | `hsd_mtx_inverse_concat`, nine cofactors | `HSD_MtxInverseConcat` | 0x803796A0, A4, AC, B0, B4, B8, DC, E0, E4 | `fmsubs` / `fnmsubs`, then separate `fmuls` | Yes |
| mtx.rs:632 | `hsd_mtx_inverse_concat`, inverse translation | `HSD_MtxInverseConcat` | 0x80379704/18, 0x8037970C/1C, 0x80379714/20 | `fmsubs` + `fnmsubs` per row | Yes |
| mtx.rs:640 | `hsd_mtx_inverse_concat`, twelve concatenated entries | `HSD_MtxInverseConcat` | 0x80379748–0x80379888 (aliased); 0x803798AC–0x803799EC (distinct) | 2 `fmadds` per entry; translation finishes with `fadds` | Yes |
| mtx.rs:672 | `hsd_mtx_inverse_transpose`, nine cofactors | `HSD_MtxInverseTranspose` | 0x80379AF0, 0x80379B10, 30, 50, 70, 90, B0, D0, F0 | Alternating `fmsubs` / `fnmsubs`, then separate `fmuls` | Yes |
| mtx.rs:713 | `hsd_mtx_get_rotation`, length0 | `HSD_MtxGetRotation` | 0x80379C50–0x80379C68 | 3 `fmuls`, 2 `fadds` | No |
| mtx.rs:716 | `hsd_mtx_get_rotation`, length1 | `HSD_MtxGetRotation` | 0x80379CD4–0x80379CEC | 3 `fmuls`, 2 `fadds` | No |
| mtx.rs:719 | `hsd_mtx_get_rotation`, length2 | `HSD_MtxGetRotation` | 0x80379D58–0x80379D70 | 3 `fmuls`, 2 `fadds` | No |
| mtx.rs:821 | `hsd_mk_rotation_mtx`, m01/m11/m02/m12 | `HSD_MkRotationMtx` | 0x8037A1B4, 0x8037A1C4, 0x8037A1D8, 0x8037A1E0 | `fmsubs`, `fmadds`, `fmadds`, `fmsubs` | Yes |
| mtx.rs:882 | `hsd_mtx_srt`, m01/m11/m02/m12 | `HSD_MtxSRT` | 0x8037A388, 0x8037A38C, 0x8037A3B4, 0x8037A3B8 | `fmsubs`, `fmadds`, `fmadds`, `fmsubs`; separate scale multiply | Yes |
| mtx.rs:947 | `hsd_mtx_scaled_add`, all entries | `HSD_MtxScaledAdd` | 0x8037A554–0x8037A604, stride 0x10 | 12 `fmadds` | Yes |
| quat.rs:9 | Module overview | Quaternion routines | See rows below | Mixed scalar arithmetic | Mixed; note updated |
| quat.rs:56 | `mat_to_quat`, three column lengths | `MatToQuat` | 0x8037E73C–0x8037E74C; 0x8037E7B4–0x8037E7C8; 0x8037E82C–0x8037E844 | 3 `fmuls`, 2 `fadds` per column | No |
| quat.rs:106 | `mtx_to_euler`, xy length | `HSD_QuatLib_8037EB28` | 0x8037EB50–0x8037EB5C | 2 `fmuls`, 1 `fadds` | No |
| quat.rs:124 | `quat_mul`, x/y/z/w | `HSD_QuatLib_8037EC4C` | 0x8037EC94–0x8037ECC4 | xyz each `fmadds` + `fmsubs` + separate `fadds`; w has 2 `fmadds` + `fmsubs` | Yes |
| quat.rs:142 | `quat_from_axis_angle`, axis length | `HSD_QuatLib_8037ECE0` | 0x8037ED0C–0x8037ED24 | 3 `fmuls`, 2 `fadds` | No |
| quat.rs:170 | `euler_to_quat`, w/x/y/z | `EulerToQuat` | 0x8037EED0, 0x8037EED8, 0x8037EEE8, 0x8037EEF0 | `fmadds`, `fmsubs`, `fmadds`, `fmsubs` | Yes |
| quat.rs:189 | `quat_slerp`, cosom | `HSD_QuatLib_8037EF28` | 0x8037EF7C, 0x8037EF88, 0x8037EF94 | 3 `fmadds` | Yes |
| quat.rs:204 | `quat_slerp`, general/near-equal blend | `HSD_QuatLib_8037EF28` | 0x8037F000–0x8037F03C, stride 0x14 | 4 `fmadds` | Yes |
| quat.rs:219 | `quat_slerp`, opposite blend t < 0.5 | `HSD_QuatLib_8037EF28` | 0x8037F0B4–0x8037F0F0, stride 0x14 | 4 `fmadds` | Yes |
| quat.rs:229 | `quat_slerp`, opposite blend t >= 0.5 | `HSD_QuatLib_8037EF28` | 0x8037F13C–0x8037F178, stride 0x14 | 4 `fmadds` | Yes |
| fobj.rs:57 | Module overview | `FObjUpdateAnim`, `splGetHelmite` | See four rows below | 4 `fmadds` | Yes; note updated |
| fobj.rs:426 | `spl_get_helmite`, p0/p1 blend | `splGetHelmite` (spline.c) | 0x80378A84 | `fmadds f0, f3, f1, f0` | Yes |
| fobj.rs:428 | `spl_get_helmite`, d0 term | `splGetHelmite` | 0x80378A88 | `fmadds f0, f5, f2, f0` | Yes |
| fobj.rs:430 | `spl_get_helmite`, d1 term | `splGetHelmite` | 0x80378A8C | `fmadds f1, f6, f9, f0` | Yes |
| fobj.rs:731 | `FObj::update_anim`, linear interpolation | `FObjUpdateAnim` | 0x8036AF98 | `fmadds f0, f2, f1, f0` | Yes |
| mtx_oracle.rs:10 | Oracle overview | mtx.c, quatlib.c | Matrix/quaternion rows above | Retail copies spell fused operations explicitly | Mixed; note updated |
| anim_ref_oracle.rs:8 | Oracle overview | fobj.c, spline.c | 0x8036AF98, 0x80378A84/88/8C | Explicit `fmadds` in retail copies | Yes; note updated |

## Sweep differences

These counts compare the same input records between the two C builds by raw
output bits (including NaN payload changes). A matrix/quaternion input counts
once if any output word differs. SDK kernels, inverse-trig stand-ins, and retail
MSL support remain identical between variants. Both oracle test binaries passed.

| Matrix/quaternion operation | Changed / inputs |
|---|---:|
| hsd_inverse | 3,440 / 3,751 |
| hsd_inverse_concat | 3,533 / 3,751 |
| hsd_inverse_transpose | 3,279 / 3,751 |
| hsd_mk_rotation | 4,090 / 7,212 |
| hsd_srt | 2,333 / 4,000 |
| hsd_scaled_add | 2,906 / 3,751 |
| quat_mul | 2,040 / 2,807 |
| euler_to_quat | 4,218 / 7,212 |
| quat_slerp | 3,189 / 6,500 |
| Other 23 operations | 0 / 86,971 |
| **Matrix/quaternion total** | **29,028 / 129,706** |
| Hermite | 18,070 / 60,000 |
| FObj streams | 1,998 / 4,011 |
| FObj steps within those streams | 13,925 / 120,437 |

Rust versus retail C has zero mismatches under the existing comparison rules.
The matrix oracle separately excludes NaN payload differences: 24 words in
mtx_inverse, 23 in mtx_invxpose, 123 in mtx_quat, 373 in hsd_inverse, 700 in
hsd_inverse_concat, 243 in hsd_inverse_transpose, and 114 in hsd_mtx_quat
(1,600 words total). This audit preserves that pre-existing comparison policy;
it does not establish arbitrary-NaN payload equality. Hermite and FObj have zero
excluded differences. No test expectations or tolerances were changed.

## Findings and boundaries

- The determinant rounds its **second** triple product first, then fuses the
  first product into it, then the third. Its three subtracting terms use
  `fnmsubs`, including the rounded-result negation and its signed-zero behavior.
- Scalar length calculations do not fuse their squares. The `fnmsub` hits in
  those functions belong to MSL's inlined double-precision square root. The
  oracle driver now calls `gekko_fnmsub` for all three Newton steps, matching
  gekko-math. Its SDK helpers also use the shared header, including the volatile
  rounded intermediate required by negated forms on the host compiler.
- The quaternion product preserves separate final adds for x/y/z. Slerp's
  opposite branch still writes and then overwrites its perpendicular quaternion.
  A fused blend can leave a rounding residual at equal weights, where the old
  unfused formula cancels to exact zero.
- Hermite's basis calculation remains separate multiply/add/subtract operations;
  only its three weighted accumulations fuse. The linear FObj update adds one
  more fused operation.
- `PSMTXConcat` was already correctly transcribed: 0x8034224C/54/5C/64 use
  `ps_madds1`; 0x80342270/74/78/7C use `ps_madds0`; 0x80342288/94/B8 apply
  `(0,1) * a[row][3]` with `ps_madds1`. Each lane uses the selected scalar
  from frC. In particular column 2 really adds `0 * translation` and can
  become NaN for infinite translation. The C driver already spells both lanes.
- `PSMTXQuat` at 0x803426FC uses `ps_madds0` with `(x,y)` and the low lane
  `z` of `(z,w)`, producing `(x*z+y*w, y*z+x*w)`; 0x80342704 uses `ps_nmsub`
  to subtract twice the respective w products. Existing Rust/C lane formulas
  match; no new PS arithmetic transcription was necessary.
- The animation oracle actually compiles **fobj.c and spline.c**, not aobj.c.
  AObj control flow remains covered by anim_aobj.rs; its modulo implementation
  uses the already audited gekko-math function.
- Native twins validate transcription, not independent execution of the retail
  binary. Inverse trig remains StubTrig in this oracle. The existing SDK
  double-estimate `fmuls` frC truncation limitation is outside this audit.

## Changed files

- src/mtx.rs, src/quat.rs, src/fobj.rs: audited arithmetic and address comments.
- tests/mtx_oracle.rs, tests/anim_ref_oracle.rs: retail/verbatim builds, drift
  checks, comparison counts, isolated per-test build/output directories.
- tests/ref/mtx/driver.c: shared FMA helpers and fused double sqrt refinement.
- tests/ref/mtx/{mtx.c,quatlib.c}: verbatim submodule copies for drift checks.
- tests/ref/mtx/retail/{mtx.c,quatlib.c}: explicit retail fusion.
- tests/anim_ref/retail/{fobj.c,spline.c}: explicit retail fusion.
- tests/ref/FUSION_AUDIT.md: this inventory and evidence.
- docs/ASM.md: short hsd-anim results section.

The C copies derive from the pinned doldecomp/melee submodule. The files outside
retail remain verbatim and are checked byte-for-byte against that submodule.
Only audited functions in the retail copies are claimed retail-faithful; the
unused remainder of spline.c has not been audited here.

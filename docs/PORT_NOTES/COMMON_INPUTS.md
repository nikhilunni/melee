# Common inputs in Fox/Marth on Final Destination

Twenty-four directed scenarios cover both fighters: idle/dash/run taunts, early
dash forward escape, middle/late dash shield, run shield, standing shield grab,
late dash/run shield grab, C-stick shield jump, and delayed powershield. Each has
300 retail ticks with RNG and ordered particles and 150 ticks of both fighters'
local bone transforms. Raw assertions include the dash item-throw and grab windows.

Taunt entry follows the character table and checks authored left-animation
availability. Both characters use AppealSR with their available motion 239; input
unlocks only the retail attack/defense prefix. Fox uses animation-driven ground
motion. Dash defense preserves the shared successful-interrupt FMA friction tail;
Run defense initializes its own item-throw window without that Dash tail.

The field formerly named jump_delay is a dash item-throw countdown. The consumer
ftCo_8009515C uses an alias of the guard union and decrements it even with empty
hands. Grab uses the separate window at 2364. C-stick jump retains its input source
for short-hop release. Delayed powershield preserves guard scratch and owned
graphics while replacing reflection windows; SkipAnim retains the incoming clock
minus its rate, as Fighter_ChangeMotionState80069C0C specifies.

Bone coverage exposed two existing transition defects. Full neutral shield pose
must restore scale/translation on dynamics-owned joints, retaining their rotation
(ftAnim8006FA58 and lb8000B6A4/B760). A departing motion can also request that one
joint bypass the next animation blend (Fighter_ChangeMotionState80069C48..80069FAC).
That copy preserves translation/rotation and clears quaternion ownership, leaving
scale to normal blending. Both fixes preserve captured expectations.

Debug and release workspace gates each pass 1,307 tests with zero failures and
one existing ignore. All 24 cases also pass the zero-allocation gate. All-target
clippy, formatting, 220 harness tests and schema checks pass. Oracle files and the
read-only diagnostic pose probe are ignored and backed up outside the repository.

The caller-owned exploration example saves complete cold-start input recordings
and action/transition reports. Version 1 uses eight seeds, both port assignments,
three input profiles and a 6000-tick limit per case. Choices use separate RNG and
hold durations; the simulation RNG is untouched. Saved failures reproduce through
melee-replay. These generated runs establish robustness coverage only, not retail
exactness or human playtesting. The first run had 46 faulted cases and 2 cases that
finished by stock elimination; its failures include Run defense/taunt, ledge/capture,
Fire hit effects and contact handling. The broader matchup remains open.

The native build passes. The current app smoke aborts before gameplay because
macOS IOSurface reports 1,020 graphics clients (922 attributed to Safari); new
Dolphin launches fail at the same host limit. This is an environment-limited
check, not a passed native smoke. The previous recovery packet's 234-tick export
is reused only as a headless replay control. Stripped simulator: 3,943,848 bytes,
+280 versus the recovery packet; no new full performance-census claim.

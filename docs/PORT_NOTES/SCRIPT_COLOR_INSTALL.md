# Script color installation before the next program step

Retail ftAction_80071A14 calls ftColl_8007B62C, which installs a color through
ftCo_800BFFD0/lb_800144C8 immediately. Installation changes ownership, priority
and program pointer; it does not run ftCo_800C0408 or advance a program timer.
The port already mapped body status to the correct color request, but deferred
installation until the next color step. Input-phase script entries therefore
exposed the old color owner until the next Animation phase.

The thrower's direct protection grant also uses the existing install-now helper
after both entry scripts, preserving its queue drain and delaying program
execution until the ordinary color proc. The same280tick capture proves that
Fox has color9/invincibility8/timer0 at120 and the first program step at121.

Manual revival exits use the same direct ftColl_8007B7A4 install after
selected entry; timeout installs before Fall. Complete shield, analog-shield,
held-shield and Rollout captures compare protection counters and primary owners
on every row and require entry timer0/no executed color, then timer1/color on
the next tick. The shared protection helper retains each caller's signed max
and placement relative to motion entry. Floor contact is the dependent caller.

Script batches now synchronize the existing ordered install helper after the
normal step, motion-entry step/seek/control and both idle-restart callers.
The normal color program advance remains in place. Installation allocates
nothing, executes no sound/graphics and draws no RNG. Direct install/clear
paths keep their existing request drains. No intermediate script command
reads color slot ownership; batched request order and static table priority
are preserved. This changes ownership publication, not the color interpreter.

The existing complete280-tick getup_attack_fd_marth recording shows Marth
entering DownAttackU187 at195, with primary color2, nonzero program pointer
and timer0. At196 the ordinary Animation proc has executed it (timer4); the
program clears at230. The actual simulation regression compares both fighters'
primary owners on all280raw rows and asserts installed/unstepped at195,
execution at196 and clearing at230. It never treats stale RGBA storage on an
inactive program as active color evidence.

A separate complete1679-tick revival-floor recording exposed this gap at817
and provides further priority/replacement coverage: damage14 survives earlier
getup entry, color2 appears817..847 and clears848, later color2 and damage4
precede final revival color9. That record's final floor callback is a distinct
dependent change; its entry-only limit remains documented separately.

Raw and assembly audit: MAIN target/wave17/revival-color-search/plan.md,
raw-transitions.json and install/clear/body-status.asm.txt. Combined acceptance
logs and exact commit/base provenance are preserved in MAIN
 target/wave17/revival-floor/. No new recordings or protected data writes.

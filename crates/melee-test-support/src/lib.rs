//! Shared integration-test preflight; never a dependency of gameplay code.
use std::{env, path::Path};

pub mod rendered_pose;

/// Record from the main checkout; this helper never launches Dolphin.
pub const M2_CAPTURE_COMMAND: &str = r#"MELEE_BONES_SAVESTATE="$PWD/harness/roms/idle_ys_fox.sav" MELEE_BONES_OUT="$PWD/harness/traces/fox_ys.bones.expected.jsonl" MELEE_BONES_FRAMES=130 MELEE_BONES_FIGHTER_INDEX=0 MELEE_BONES_ANY_ANIM=1 "$HOME/Projects/dolphin-scripting/build/Binaries/Dolphin.app/Contents/MacOS/Dolphin" -v OGL -C Dolphin.Core.SIDevice0=6 -C Dolphin.Core.SIDevice1=6 -e "$PWD/harness/roms/GALE01.iso" --script "$PWD/harness/dolphin_bones_snippet.py""#;

/// Check required files in caller order. Missing data fails the test unless
/// `MELEE_ALLOW_MISSING_DATA=1`; only that opt-in returns `false` to the caller.
///
/// `MELEE_TEST_DATA_ROOT` replaces `harness/` for this preflight only, allowing
/// absence tests without moving real data. It does not redirect asset reads.
#[track_caller]
pub fn require_files(paths: impl IntoIterator<Item = impl AsRef<Path>>) -> bool {
    let override_root = env::var_os("MELEE_TEST_DATA_ROOT");
    for path in paths {
        let original = path.as_ref();
        let resolved = override_root.as_ref().and_then(|root| {
            let mut components = original.components();
            components
                .find(|part| part.as_os_str() == "harness")
                .map(|_| Path::new(root).join(components.as_path()))
        });
        let path = resolved.as_deref().unwrap_or(original);
        if path.is_file() {
            continue;
        }
        let recovery = recovery_command(original);
        let message = format!(
            "missing oracle data: {}; restore with {recovery}; only contributors without local data may opt out with MELEE_ALLOW_MISSING_DATA=1",
            path.display()
        );
        if env::var_os("MELEE_ALLOW_MISSING_DATA").is_some_and(|value| value == "1") {
            eprintln!("{message}");
            return false;
        }
        panic!("{message}");
    }
    true
}

fn recovery_command(path: &Path) -> String {
    if path.components().any(|part| part.as_os_str() == "roms") {
        return "the disc extraction/savestate instructions in docs/DISC.md".into();
    }
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    if name.ends_with("_probe.jsonl") {
        return if path.components().any(|part| part.as_os_str() == "jit") {
            "`GEKKO_PROBE_OUT=harness/traces/jit harness/gekko_probe/run.sh 4`".into()
        } else {
            "`harness/gekko_probe/run.sh 0`".into()
        };
    }
    let scenario = name.split('.').next().unwrap_or_default();
    if scenario == "fox_ys" {
        return format!("`{M2_CAPTURE_COMMAND}`");
    }
    if let Some(player) = name
        .split(".bones_vi_p")
        .nth(1)
        .and_then(|s| s.chars().next())
    {
        return format!("the bone capture command in docs/M2_GATE.md with MELEE_BONES_ANY_ANIM=1 MELEE_BONES_FRAMES=130 MELEE_BONES_FIGHTER_INDEX={player} MELEE_BONES_SAVESTATE=harness/roms/{scenario}.sav MELEE_BONES_OUT=harness/traces/{scenario}.bones_vi_p{player}.jsonl");
    }
    let ledger = if name.contains(".ledger600.") {
        " --ledger-suffix ledger600"
    } else {
        ""
    };
    let bones = if name.contains(".bones.") {
        " --bones <required-ticks>"
    } else {
        ""
    };
    format!("`uv run --project harness python harness/record.py harness/scenarios/{scenario}.toml{ledger}{bones}` (see docs/DOLPHIN_RUN.md)")
}

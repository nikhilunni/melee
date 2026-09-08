//! Milestone 2: the unfiltered Fox Wait1 skeleton with an identity world
//! transform. See docs/M2_GATE.md for capture timing and the retail fighter
//! position/facing/scale overrides deliberately outside this gate.

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{ensure, Context, Result};
use hsd_anim::jobj::{JObjId, JObjTree};
use hsd_anim::load::load_joint_tree;
use hsd_anim::mtx::InverseTrig;
use hsd_archive::desc::{read_public_figatree, read_public_jobj};
use hsd_archive::Archive;
use melee_diff::{Record, Value};
use melee_ft::desc::{read_fox_animations, WAIT1_ANIMATION_INDEX};
use melee_lb::anim::{attach_figatree, request_frame};

/// Resolve independently of the caller's working directory.
pub fn default_assets() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files")
}

/// Melee supplies these HSD callbacks; hsd-anim cannot depend on melee-lb.
struct RetailTrig;

impl InverseTrig for RetailTrig {
    fn atan2f(y: f32, x: f32) -> f32 {
        melee_lb::trigf::atan2f(y, x)
    }
    fn asinf(x: f32) -> f32 {
        melee_lb::trigf::asinf(x)
    }
    fn acosf(x: f32) -> f32 {
        melee_lb::trigf::acosf(x)
    }
}

fn read_asset(assets: &Path, name: &str) -> Result<Vec<u8>> {
    let path = assets.join(name);
    std::fs::read(&path).with_context(|| format!("reading {}", path.display()))
}

fn load_fox_wait1(assets: &Path) -> Result<(JObjTree, JObjId)> {
    let costume = Archive::parse(&read_asset(assets, "PlFxNr.dat")?)?;
    let descriptor = read_public_jobj(&costume, "PlyFox5K_Share_joint")?;
    let (mut tree, root) = load_joint_tree(&costume, &descriptor)?;
    ensure!(tree.next(root).is_none(), "Fox skeleton must have one root");
    let fighter = Archive::parse(&read_asset(assets, "PlFx.dat")?)?;
    let table = read_fox_animations(&fighter)?;
    let wait = &table.entries[WAIT1_ANIMATION_INDEX];
    let aj = read_asset(assets, "PlFxAJ.dat")?;
    let archive = Archive::parse(wait.sub_archive(&aj)?.context("Wait1 has no archive")?)?;
    let symbol = wait.symbol_name.as_deref().context("Wait1 has no public")?;
    let animation = read_public_figatree(&archive, symbol)?;
    attach_figatree(&mut tree, root, &animation)?;
    Ok((tree, root))
}

fn f32_value(value: f32) -> Value {
    // Match harness/jobjdump.py: JSON must remain readable even for NaNs.
    if value.is_finite() {
        Value::f32(value)
    } else {
        Value::F32 {
            bits: value.to_bits(),
            approx: 0.0,
        }
    }
}

/// The same singleton-state records as harness/jobjdump.py::records.
/// Matrices are row-major; rotation includes the unused Euler-mode w word.
fn joint_records(frame: u64, bone: usize, joint: &hsd_anim::jobj::JObj) -> Vec<Record> {
    let rotation = [
        joint.rotate.x,
        joint.rotate.y,
        joint.rotate.z,
        joint.rotate.w,
    ];
    let scale = [joint.scale.x, joint.scale.y, joint.scale.z];
    let translation = [joint.translate.x, joint.translate.y, joint.translate.z];
    [
        ("mtx", joint.mtx.0.as_flattened()),
        ("rotate", rotation.as_slice()),
        ("scale", scale.as_slice()),
        ("translate", translation.as_slice()),
    ]
    .into_iter()
    .flat_map(|(field, values)| {
        values.iter().enumerate().map(move |(i, &value)| Record {
            frame,
            phase: "bones".into(),
            state: [(format!("p0.bone[{bone}].{field}[{i}]"), f32_value(value))].into(),
        })
    })
    .collect()
}

/// What the fighter layer writes onto the skeleton before matrices are built.
/// With `None` everywhere the root keeps its archive SRT (an identity world
/// boundary), which is *not* what a fighter in a match looks like.
#[derive(Debug, Clone, Default)]
pub struct FighterPose {
    /// `HSD_JObjSetTranslate(GET_JOBJ(gobj), &fp->cur_pos)` (fighter.c:519).
    pub position: Option<hsd_types::Vec3>,
    /// `ftPartSetRotY(fp, 0, M_PI_2 * fp->facing_dir)` (fighter.c:1174):
    /// the product is a double, rounded once to f32.
    pub facing_dir: Option<f32>,
    /// `Fighter_UpdateModelScale` (fighter.c:213-229): uniform
    /// `ftCommon_GetModelScale(fp)` unless `x34_scale.z != 1.0`, which
    /// replaces only x. Fox is 0.96.
    pub model_scale: Option<f32>,
    /// Extra per-bone uniform scales observed in retail (e.g. bone 67 at
    /// 1/model_scale on Fox). TODO(meaning): find the fighter.c site that
    /// writes these; until then they are supplied by the caller.
    pub bone_scales: Vec<(usize, f32)>,
}

impl FighterPose {
    fn apply(&self, tree: &mut JObjTree, root: JObjId, bones: &[JObjId]) -> Result<()> {
        if let Some(position) = self.position {
            tree.set_translate(root, &position);
        }
        if let Some(facing) = self.facing_dir {
            let rotation_y = (core::f64::consts::FRAC_PI_2 * facing as f64) as f32;
            tree.set_rotation_x(root, 0.0);
            tree.set_rotation_y(root, rotation_y);
            tree.set_rotation_z(root, 0.0);
        }
        if let Some(scale) = self.model_scale {
            tree.set_scale(root, &hsd_types::Vec3::new(scale, scale, scale));
        }
        for &(bone, scale) in &self.bone_scales {
            let id = *bones
                .get(bone)
                .with_context(|| format!("bone {bone} does not exist"))?;
            tree.set_scale(id, &hsd_types::Vec3::new(scale, scale, scale));
        }
        Ok(())
    }
}

/// Load Fox's neutral costume and Wait1 and emit canonical JSONL.
///
/// Request `frame` once; FIRST_PLAY evaluates exactly that time, subsequent
/// samples use AObj's normal rate-1 advancement (including its loop behavior).
/// Record.frame is the sample ordinal 0..frames, not animation time.
///
/// No synthetic parent is inserted: a root with parent=None is HSD's identity
/// world boundary. Keep the archive/animation SRT, without fighter position,
/// facing rotation or model scale overrides. `setup_matrix(root)` alone does
/// not recurse downward, so call it in preorder for every joint, equivalent
/// to HSD_JObjGetMtxPtr on each bone (jobj.h:697-701).
pub fn write_fox_wait1_bones(
    assets: &Path,
    frame: f32,
    frames: u64,
    pose: &FighterPose,
    mut out: impl Write,
) -> Result<()> {
    ensure!(
        frame.is_finite() && frame >= 0.0,
        "frame must be finite and nonnegative"
    );
    ensure!(frames > 0, "frames must be positive");
    let (mut tree, root) = load_fox_wait1(assets)?;
    let ids: Vec<_> = tree.depth_first(root).collect();
    pose.apply(&mut tree, root, &ids)?;
    request_frame(&mut tree, root, frame);
    for sample in 0..frames {
        tree.anim_all::<RetailTrig>(root);
        for &id in &ids {
            tree.setup_matrix(id);
        }
        for (bone, &id) in ids.iter().enumerate() {
            for record in joint_records(sample, bone, tree.get(id)) {
                serde_json::to_writer(&mut out, &record)?;
                writeln!(out)?;
            }
        }
    }
    out.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_preserves_signed_zero_and_nonfinite_bits() {
        let mut joint = hsd_anim::jobj::JObj::init();
        joint.mtx.0[0] = [
            -0.0,
            f32::from_bits(0x7f80_1234),
            f32::INFINITY,
            f32::NEG_INFINITY,
        ];
        let records = joint_records(0, 0, &joint);
        let jsonl = records
            .iter()
            .map(|r| serde_json::to_string(r).unwrap() + "\n")
            .collect::<String>();
        assert_eq!(melee_diff::read_trace(jsonl.as_bytes()).unwrap(), records);
        for (record, bits) in
            records
                .iter()
                .zip([0x8000_0000, 0x7f80_1234, 0x7f80_0000, 0xff80_0000])
        {
            assert!(
                matches!(record.state.values().next(), Some(Value::F32 { bits: got, .. }) if *got == bits)
            );
        }
    }
}

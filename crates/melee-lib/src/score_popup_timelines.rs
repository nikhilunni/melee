//! The Time match's score popups' animations (IfAll ScInfStc_scene_models
//! model 0, if_802F7E24), as the counts the popups' procs see.
use anyhow::Result;
use hsd_anim::load::{attach_anim_joint, load_joint_tree};
use melee_ef::score_popups::PopupTimeline;
use melee_ft::fighter::RetailTrig;

/// A popup longer than this is not a HUD popup.
const MAX_STEPS: usize = 600;

/// Each popup animation's course (`PopupKind` order: "-2", "-1", "+1").
pub(crate) fn load(archive: &hsd_archive::Archive) -> Result<[PopupTimeline; 3]> {
    let mut timelines: [PopupTimeline; 3] = Default::default();
    for (kind, timeline) in timelines.iter_mut().enumerate() {
        *timeline = timeline_of(archive, kind as u32)?;
    }
    Ok(timelines)
}

/// fn_802F77F8: load the joint, gm_8016895C(jobj, desc, kind),
/// HSD_JObjReqAnimAll(0), HSD_JObjAnimAll. Each proc then reads the first
/// AObj's frame (lbGetJObjCurrFrame), tests lb_8000B09C, and animates while
/// it holds.
fn timeline_of(archive: &hsd_archive::Archive, kind: u32) -> Result<PopupTimeline> {
    let (joint, animation) = hsd_archive::desc::model::read_dynamic_model_animation(
        archive,
        "ScInfStc_scene_models",
        0,
        kind,
    )?;
    let (mut tree, root) = load_joint_tree(archive, &joint)?;
    attach_anim_joint(&mut tree, root, &animation, archive)?;
    tree.req_anim_all(root, 0.0);
    tree.anim_all::<RetailTrig>(root);
    let mut frames = Vec::new();
    loop {
        // lbFindJObjWithAObj: the first JObj, depth first, with an AObj.
        let mut first = None;
        let mut active = false;
        tree.walk_tree(root, &mut |id, _| {
            if let Some(aobj) = &tree.get(id).aobj {
                first.get_or_insert(aobj.curr_frame);
                active |= aobj.flags & hsd_anim::aobj::AOBJ_NO_ANIM == 0;
            }
        });
        frames.push(first.unwrap_or(0.0));
        if !active {
            break;
        }
        anyhow::ensure!(frames.len() <= MAX_STEPS, "score popup {kind} never ends");
        tree.anim_all::<RetailTrig>(root);
    }
    Ok(PopupTimeline { frames })
}

//! Compact fighter animation attachment (`melee/lb/lbanim.c`).
//!
//! The whole-tree helper covers a matching, unfiltered skeleton such as Fox's
//! Wait1. Retail `ftAnim_8006F4C8` selects joints through fighter part flags;
//! remapping, partial-body animation and blending remain fighter-layer work.
//! HSD itself has no FigaTree helpers; request/interpret use its existing runtime.
//!
//! Translation-filtered attachment preserves retail's accepted-prefix behavior;
//! see [`attach_joint_tracks_without_translation`].

use std::{collections::HashSet, fmt};

use hsd_anim::aobj::AObj;
use hsd_anim::fobj::FObj;
use hsd_anim::jobj::{jobj_sort_anim, JObjId, JObjTree, JOBJ_CLASSICAL_SCALE, JOBJ_INSTANCE};
use hsd_archive::desc::{FigaTrack, FigaTree};

/// Invalid inputs to the FigaTree/skeleton attachment boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachError {
    InvalidJoint(JObjId),
    RepeatedJoint(JObjId),
    NodeCount { joints: usize, nodes: usize },
    InvalidTracks,
    NoAcceptedTracks,
}

impl fmt::Display for AttachError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJoint(id) => write!(f, "joint {id:?} is outside the tree"),
            Self::RepeatedJoint(id) => write!(f, "joint {id:?} occurs more than once"),
            Self::NodeCount { joints, nodes } => {
                write!(
                    f,
                    "FigaTree has {nodes} nodes but skeleton subtree has {joints} joints"
                )
            }
            Self::InvalidTracks => f.write_str("FigaTree track counts or stream lengths disagree"),
            Self::NoAcceptedTracks => {
                f.write_str("translation filter did not allocate a valid FObj")
            }
        }
    }
}

impl std::error::Error for AttachError {}

/// `lbAnim_InitFrames` (`lbanim.c`, retail `0x8001E560`): copy one track
/// into the zeroed HSD runtime state. No floating-point arithmetic in retail.
fn load_track(track: &FigaTrack) -> FObj {
    FObj::from_shared_stream(
        track.ad.clone(),
        u32::from(track.length),
        // retail 0x8001E5A0/0x8001E5AC: lha + sth, signed 16-bit bit copy.
        // All i16 values are exactly representable by the descriptor's f32.
        f32::from(track.startframe as i16),
        track.obj_type,
        track.frac_value,
        track.frac_slope,
    )
}

/// `fn_8001E60C` / `lbAnim_8001E7E8` (`lbanim.c`, 0x8001E60C/0x8001E7E8).
/// Retail 0x8001E6A0 increments the track pointer inside the accepted branch;
/// 0x8001E6AC increments the loop counter unconditionally. On the first type
/// 5, 6 or 7, every remaining iteration inspects that SAME rejected track.
/// Thus only the prefix before it is attached, not all nontranslation tracks.
/// If that prefix is empty, C's final FObj pointer is uninitialized; retail
/// 0x8001E6BC stores through r3, still the incoming track pointer. Return an
/// error for that invalid/corrupting input instead of inventing an empty
/// animation. A zero-count node, in contrast, is a defined no-op.
pub fn attach_joint_tracks_without_translation(
    tree: &mut JObjTree,
    joint: JObjId,
    animation: &FigaTree,
    tracks: &[FigaTrack],
) -> Result<(), AttachError> {
    if tracks.is_empty() {
        return Ok(());
    }
    let accepted = tracks
        .iter()
        .position(|t| matches!(t.obj_type, 5..=7))
        .unwrap_or(tracks.len());
    if accepted == 0 {
        return Err(AttachError::NoAcceptedTracks);
    }
    attach_joint_tracks(tree, joint, animation, &tracks[..accepted]);
    Ok(())
}

/// `lbAnim_8001E6D8` (`lbanim.c`, retail `0x8001E6D8`), including its inline
/// `lbAnim_JObjSortAnim` (`0x8001E758-0x8001E7A8`). Replace one joint's AObj,
/// move the first TYPE_JOBJ track to the front, and apply classical scaling.
/// Empty track lists leave both the previous AObj and scale flags unchanged.
/// The joint id must be valid and tracks must come from a parsed FigaTree.
pub fn attach_joint_tracks(
    tree: &mut JObjTree,
    joint: JObjId,
    animation: &FigaTree,
    tracks: &[FigaTrack],
) {
    if tracks.is_empty() {
        return;
    }
    let mut aobj = AObj::alloc();
    aobj.set_flags(animation.flags);
    aobj.set_rewind_frame(0.0);
    aobj.set_end_frame(animation.frames);
    let mut runtime_tracks = tree.take_animation_tracks(joint);
    runtime_tracks.clear();
    runtime_tracks.extend(tracks.iter().map(load_track));
    aobj.set_fobj(runtime_tracks);
    jobj_sort_anim(&mut aobj);
    tree.get_mut(joint).aobj = Some(aobj);
    if animation.type_ & 1 != 0 {
        tree.set_flags(joint, JOBJ_CLASSICAL_SCALE);
    } else {
        tree.clear_flags(joint, JOBJ_CLASSICAL_SCALE);
    }
}

/// Compose `lbAnim_8001E6D8` (`0x8001E6D8`) in `HSD_JObjAddAnimAll` order
/// (`jobj.c`, `0x8036FB5C`): root, children, then their siblings. Root siblings
/// and instance children are excluded. This is the unfiltered counterpart of
/// `ftAnim_8006F4C8` (`ftanim.c`, `0x8006F4C8`), not a port of its part masks.
/// Validates the entire mapping before mutation. Returns nodes consumed,
/// including zero-track nodes (which do not receive an AObj).
pub fn attach_figatree(
    tree: &mut JObjTree,
    root: JObjId,
    animation: &FigaTree,
) -> Result<usize, AttachError> {
    let joints = subtree_joints(tree, root)?;
    if joints.len() != animation.nodes.len() {
        return Err(AttachError::NodeCount {
            joints: joints.len(),
            nodes: animation.nodes.len(),
        });
    }
    let total = animation.nodes.iter().try_fold(0usize, |sum, &count| {
        sum.checked_add(usize::try_from(count).ok()?)
    });
    if total != Some(animation.tracks.len())
        || animation
            .tracks
            .iter()
            .any(|t| t.ad.len() != usize::from(t.length))
    {
        return Err(AttachError::InvalidTracks);
    }
    for (&joint, tracks) in joints.iter().zip(animation.tracks_by_node()) {
        attach_joint_tracks(tree, joint, animation, tracks);
    }
    Ok(joints.len())
}

/// `HSD_JObjAddAnimAll` (`jobj.c`, retail `0x8036FB5C`): subtree preorder,
/// bounded by checked ids and a visited set because runtime links are public.
fn subtree_joints(tree: &JObjTree, root: JObjId) -> Result<Vec<JObjId>, AttachError> {
    let mut pending = vec![(root, false)];
    let mut visited = HashSet::new();
    let mut joints = Vec::new();
    while let Some((id, follow_next)) = pending.pop() {
        if id.0 >= tree.len() {
            return Err(AttachError::InvalidJoint(id));
        }
        if !visited.insert(id) {
            return Err(AttachError::RepeatedJoint(id));
        }
        let joint = tree.get(id);
        joints.push(id);
        if let Some(next) = joint.next.filter(|_| follow_next) {
            pending.push((next, true));
        }
        if let Some(child) = joint.child.filter(|_| joint.flags & JOBJ_INSTANCE == 0) {
            pending.push((child, true));
        }
    }
    Ok(joints)
}

/// `HSD_JObjReqAnimAll` (`jobj.c`, retail `0x8036F8BC`): request a frame
/// throughout the subtree. The next `JObjTree::anim_all` evaluates that frame;
/// later calls advance at each AObj's rate. The root must be a valid joint id.
pub fn request_frame(tree: &mut JObjTree, root: JObjId, frame: f32) {
    tree.req_anim_all(root, frame);
}

/// `lbAnim_8001E8F8` (`lbanim.c`, retail `0x8001E8F8`): null has zero frames.
pub fn animation_frames(animation: Option<&FigaTree>) -> f32 {
    animation.map_or(0.0, |tree| tree.frames)
}

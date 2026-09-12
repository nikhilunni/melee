//! efAsync/efSync caller requests and outgoing-pose flush storage.
use hsd_types::{Mtx, Vec3};
use melee_types::fixed::FixedVec;
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EffectRequest {
    /// efAsync kind 2 / 0x41D: owned model 0xF at a fixed world origin.
    WallJump {
        position: Vec3,
    },
    /// S3: parameters consumed by an owned model's post-animation update callback.
    OwnedRotation {
        model: u32,
        rotation: Vec3,
    },
    // S6: ftColl_80076CBC, efSync_Spawn(27) at the physical powershield contact.
    PowershieldSpark {
        position: Vec3,
    },
    /// S6: efAsync kind 0 / effect 1051, shield-joint translation and local scale.
    ShieldBreak {
        bone: usize,
        scale: f32,
    },
    /// S6: efAsync 0x429, attached generator 0xCE and its shared AppSRT.
    DizzyStars {
        bone: usize,
        scale: f32,
    },
    // S3: synchronous efAlt generator with a live fighter joint.
    SyncAttached {
        id: u16,
        bone: usize,
    },
    /// ftColl_80078538: severity-dependent draw after the primary hit spark.
    /// ftYs_Init_8012BE3C, efSync_Spawn 0x4CF: positional shell burst.
    EggShell {
        bone: usize,
        scale: f32,
    },
    /// efSync_Spawn(1030): fixed world origin and floor-relative Z rotation.
    FireFoxRebound {
        position: Vec3,
        angle: f32,
    },
    DamageTrail {
        trajectory: f32,
    },
    /// efSync_Spawn 0x42B: the explosion at the blast-zone exit, rotated by the exit angle.
    Death {
        position: Vec3,
        angle: f32,
        scale: f32,
    },
    /// fn_800DA1D8: async kind 1, hold-bone position sampled at queue flush.
    CaptureFlash {
        bone: usize,
    },
    /// efSync_Spawn: shield model attached to the shield joint.
    Shield {
        id: u16,
        bone: usize,
    },
    /// ftColl_8007A06C -> efSync_Spawn: world-space contact effect.
    /// efSync_Spawn 1052: hitbox-pair clank midpoint.
    Clank {
        position: Vec3,
    },
    ShieldSpark {
        position: Vec3,
    },
    NormalSparkExtra {
        position: Vec3,
        facing: f32,
        variant: i32,
        random_bound: i32,
    },
    HitSpark {
        position: Vec3,
        element: melee_types::HitElement,
        damage: f32,
        large: bool,
    },
    /// ftCommon_8007DB24 -> efLib_DestroyAll: remove this fighter's owned effects.
    DestroyOwned,
    /// ftCliffCommon_80081370: async kind 2 with no bone, absolute position.
    LedgeGrab {
        position: Vec3,
    },
    /// efAsync kind 0 passes the live fighter joint without offset RNG.
    Attached {
        id: u16,
        bone: usize,
    },
    /// efAsync kinds 2/5/6 retain the bone and local offset until s_link 9.
    Graphics {
        id: u16,
        bone: usize,
        offset: Vec3,
        facing: f32,
        floor_angle: f32,
    },
    /// efAsync_Spawn(..., 3, 0x43E, root, scale), ft_0C31.c:130.
    EntryWarp {
        id: u16,
        scale: Vec3,
    },
    /// ftAction_80072E4C / ftCo_8009F834: root-relative landing dust; also the star-KO
    /// twinkle (efAsync_Spawn 0x42D at cur_pos, ftCo_DeadUpStar_Anim).
    Landing {
        id: u16,
        offset: Vec3,
        floor_angle: f32,
    },
}

/// Port capacity per fighter/proc boundary. efAsync_QueueInit (80067980)
/// uses HSD_ObjAlloc without a numeric retail cap; overflow fails explicitly.
pub const REQUEST_CAPACITY: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct QueuedEffect {
    pub request: EffectRequest,
    pub matrix: Option<Mtx>,
    after_graphics: bool,
}
impl QueuedEffect {
    fn immediate(&self) -> bool {
        self.matrix.is_some() || self.request.is_immediate()
    }
}

/// A flat queue retains sealed flushes in dispatch order, avoiding recursive
/// heap-owned batches. Deferred entries alone are reversed at their flush.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EffectQueue {
    entries: FixedVec<QueuedEffect, REQUEST_CAPACITY>,
}
impl EffectQueue {
    pub fn push(&mut self, request: EffectRequest) {
        assert!(
            self.entries.len() < REQUEST_CAPACITY,
            "effect storage capacity {REQUEST_CAPACITY} exhausted"
        );
        let index = self
            .entries
            .iter()
            .position(|e| e.after_graphics)
            .unwrap_or(self.entries.len());
        self.entries.insert(
            index,
            QueuedEffect {
                request,
                matrix: None,
                after_graphics: false,
            },
        );
    }
    /// An entry callback runs after its motion script, whose graphics await the
    /// proc's RNG boundary. Keep this request behind those graphics when resolved.
    pub fn push_after_graphics(&mut self, request: EffectRequest) {
        self.entries.push(QueuedEffect {
            request,
            matrix: None,
            after_graphics: true,
        });
    }
    pub fn finish_graphics(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.after_graphics = false;
        }
    }
    pub fn pop(&mut self) -> Option<EffectRequest> {
        self.entries.pop().map(|e| e.request)
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn clear(&mut self) {
        while self.entries.pop().is_some() {}
    }
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &EffectRequest> {
        self.entries.iter().map(|e| &e.request)
    }
    /// efAsync_QueueFlush (80067624): newest pending request first, using
    /// transforms sampled before the caller replaces its animation resources.
    pub fn resolve_pending(&mut self, mut matrix: impl FnMut(Option<usize>) -> Mtx) {
        let mut pending = FixedVec::<_, REQUEST_CAPACITY>::default();
        let mut old = std::mem::take(&mut self.entries);
        while !old.is_empty() {
            let entry = old.remove(0);
            if entry.immediate() {
                self.entries.push(entry);
            } else {
                pending.push(entry);
            }
        }
        while let Some(mut entry) = pending.pop() {
            entry.matrix = Some(matrix(entry.request.bone()));
            self.entries.push(entry);
        }
    }
    pub(crate) fn drain(
        &mut self,
        timing: crate::EffectTiming,
    ) -> FixedVec<QueuedEffect, REQUEST_CAPACITY> {
        let mut drained = FixedVec::default();
        match timing {
            crate::EffectTiming::BeforeGraphics => {
                let end = self
                    .entries
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| matches!(e.request, EffectRequest::DestroyOwned))
                    .map(|(i, _)| i + 1)
                    .last()
                    .unwrap_or(0);
                for _ in 0..end {
                    let entry = self.entries.remove(0);
                    assert!(entry.immediate(), "motion entry must seal outgoing effects");
                    drained.push(entry);
                }
            }
            crate::EffectTiming::Immediate => {
                let mut index = 0;
                while index < self.entries.len() {
                    if self.entries.iter().nth(index).unwrap().immediate() {
                        drained.push(self.entries.remove(index));
                    } else {
                        index += 1;
                    }
                }
            }
            crate::EffectTiming::Deferred => {
                // Immediate effects are drained at every preceding proc. A sealed
                // batch must reach that boundary before link 9 reverses pending work.
                assert!(
                    self.entries.iter().all(|e| !e.immediate()),
                    "undrained immediate effects"
                );
                while let Some(entry) = self.entries.pop() {
                    drained.push(entry);
                }
            }
        }
        drained
    }
}
impl<const N: usize> PartialEq<[EffectRequest; N]> for EffectQueue {
    fn eq(&self, other: &[EffectRequest; N]) -> bool {
        self.iter().eq(other.iter())
    }
}
impl EffectRequest {
    fn is_immediate(&self) -> bool {
        matches!(
            self,
            Self::OwnedRotation { .. }
                | Self::PowershieldSpark { .. }
                | Self::SyncAttached { .. }
                | Self::FireFoxRebound { .. }
                | Self::Death { .. }
                | Self::Shield { .. }
                | Self::HitSpark { .. }
                | Self::NormalSparkExtra { .. }
                | Self::ShieldSpark { .. }
                | Self::Clank { .. }
                | Self::DestroyOwned
        )
    }
    fn bone(&self) -> Option<usize> {
        match *self {
            Self::DizzyStars { bone, .. }
            | Self::ShieldBreak { bone, .. }
            | Self::EggShell { bone, .. }
            | Self::CaptureFlash { bone }
            | Self::Attached { bone, .. }
            | Self::SyncAttached { bone, .. }
            | Self::Graphics { bone, .. } => Some(bone),
            _ => None,
        }
    }
}
/// Scene-independent owner boundary. Implemented by the concrete fighter core;
/// the effect engine needs neither character callbacks nor scene ownership.
pub trait EffectOwner {
    fn effect_queue(&mut self) -> &mut EffectQueue;
    fn effect_matrix(&mut self, bone: Option<usize>) -> Mtx;
    fn effect_facing(&self) -> f32;
    fn effect_scale(&self) -> hsd_types::Vec3;
}

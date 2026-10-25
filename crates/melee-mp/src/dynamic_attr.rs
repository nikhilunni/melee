//! gr/grdynamicattr.c: circles of another floor material a stage lays on
//! its lines for a while (Flat Zone's oil spill). mpCollEnd asks
//! [`DynamicAttrs::material_at`] for every grounded collision.
use hsd_types::Vec3;

/// `grDynamicAttr_8049EFA0`'s four slots.
pub const SLOTS: usize = 4;

/// `grDynamicAttr_UnkStruct` (gr/types.h:244).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DynamicAttr {
    /// +0: the next slot in its list.
    pub next: Option<u8>,
    /// +4: the material.
    pub material: i32,
    /// +8: the centre.
    pub position: Vec3,
    /// +14: the line it lies on.
    pub line: i32,
    /// +18: its radius.
    pub radius: f32,
    /// +1C: frames left (0 or less: forever).
    pub frames: i32,
}

/// The pool: `grDynamicAttr_804D6960` (live) and `grDynamicAttr_804D6964`
/// (free) heads over the slots.
#[derive(Clone, Debug, PartialEq)]
pub struct DynamicAttrs {
    pub slots: [DynamicAttr; SLOTS],
    pub live: Option<u8>,
    pub free: Option<u8>,
}

impl Default for DynamicAttrs {
    /// grDynamicAttr_801CA0B4 (0x801CA0B4, from mpLib_80058820): each slot
    /// links to the one before; the last is the free head.
    fn default() -> Self {
        let mut slots = [DynamicAttr::default(); SLOTS];
        let mut previous = None;
        for (index, slot) in slots.iter_mut().enumerate() {
            slot.next = previous;
            previous = Some(index as u8);
        }
        Self {
            slots,
            live: None,
            free: previous,
        }
    }
}

impl DynamicAttrs {
    /// grDynamicAttr_801CA0F8 (0x801CA0F8): take the free head, push it on
    /// the live list.
    pub fn create(
        &mut self,
        material: i32,
        position: Vec3,
        line: i32,
        radius: f32,
        frames: i32,
    ) -> Option<u8> {
        assert!(line != -1, "grdynamicattr.c:55: floor_id != GC_Id_None");
        let index = self.free?;
        let slot = &mut self.slots[usize::from(index)];
        self.free = slot.next;
        slot.next = self.live;
        self.live = Some(index);
        slot.material = material;
        slot.position = position;
        slot.line = line;
        slot.radius = radius;
        slot.frames = frames;
        Some(index)
    }

    /// grDynamicAttr_801CA1C0 (0x801CA1C0). A slot found behind the head is
    /// unlinked but, as in retail, not returned to the free list.
    pub fn release(&mut self, slot: Option<u8>) {
        let Some(index) = slot else {
            return;
        };
        if self.live == Some(index) {
            self.live = self.slots[usize::from(index)].next;
        } else {
            let mut cursor = self.live;
            while let Some(at) = cursor {
                if self.slots[usize::from(at)].next == Some(index) {
                    self.slots[usize::from(at)].next = self.slots[usize::from(index)].next;
                    return;
                }
                cursor = self.slots[usize::from(at)].next;
            }
        }
        self.slots[usize::from(index)].next = self.free;
        self.free = Some(index);
    }

    /// grDynamicAttr_801CA224 (0x801CA224), from mpLib_800587FC: age every
    /// live slot; one that runs out is released.
    pub fn tick(&mut self) {
        let mut cursor = self.live;
        while let Some(index) = cursor {
            let slot = &mut self.slots[usize::from(index)];
            cursor = slot.next;
            if slot.frames > 0 {
                slot.frames -= 1;
                if slot.frames <= 0 {
                    self.release(Some(index));
                }
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.live.is_none()
    }
}

impl crate::map::CollMap {
    /// grDynamicAttr_801CA284 (0x801CA284): the material of the first live
    /// circle on a line connected to `line` that holds `position`, or 0.
    /// retail 0x801CA2E8..F8: fsubs, fmuls, fmuls, fadds; sqrtf inlined.
    pub fn dynamic_material(&self, position: &Vec3, line: i32) -> i32 {
        let mut cursor = self.dynamic_attrs.live;
        while let Some(index) = cursor {
            let slot = &self.dynamic_attrs.slots[usize::from(index)];
            if self.lines_connected(line, slot.line) {
                let dx = position.x - slot.position.x;
                let dy = position.y - slot.position.y;
                let distance = gekko_math::msl::sqrtf(dx * dx + dy * dy);
                if distance < slot.radius {
                    return slot.material;
                }
            }
            cursor = slot.next;
        }
        0
    }
}

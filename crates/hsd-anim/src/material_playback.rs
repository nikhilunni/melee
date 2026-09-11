//! Prepared material animation replacement. Compile encoded tracks at load time,
//! reserve each destination once, then switch using reusable interpreter storage.
use crate::{
    aobj::{AObj, AOBJ_NO_ANIM},
    mobj::{MObj, MatAnim},
    tobj::TObj,
};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub struct PreparedMaterial {
    clock: Option<AObj>,
    textures: Vec<PreparedTexture>,
}
#[derive(Clone, Debug, PartialEq)]
struct PreparedTexture {
    id: u32,
    clock: Option<AObj>,
    images: Arc<[Option<u32>]>,
    palettes: Arc<[Option<u32>]>,
}
pub struct TextureTables<'a> {
    pub images: &'a [Option<u32>],
    pub palettes: &'a [Option<u32>],
}
impl PreparedMaterial {
    pub fn new(animation: &MatAnim) -> Self {
        Self {
            clock: animation.aobjdesc.as_ref().map(AObj::load_desc),
            textures: animation
                .textures
                .iter()
                .map(|t| PreparedTexture {
                    id: t.id,
                    clock: t.animation.as_ref().map(AObj::load_desc),
                    images: Arc::clone(&t.images),
                    palettes: Arc::clone(&t.palettes),
                })
                .collect(),
        }
    }
    /// Image tables used by a texture map, for preloading visual resources.
    pub fn texture_tables(&self, id: u32) -> Option<TextureTables<'_>> {
        self.textures
            .iter()
            .find(|t| t.id == id)
            .map(|t| TextureTables {
                images: &t.images,
                palettes: &t.palettes,
            })
    }
    pub fn set_loop(&mut self, looping: bool) {
        for clock in self
            .clock
            .iter_mut()
            .chain(self.textures.iter_mut().filter_map(|t| t.clock.as_mut()))
        {
            if looping {
                clock.flags |= crate::aobj::AOBJ_LOOP;
            }
        }
    }
    pub fn reserve(&self, material: &mut MObj) {
        reserve_clock(&mut material.aobj, self.clock.as_ref());
        for texture in &mut material.textures {
            if let Some(source) = self.textures.iter().find(|t| t.id == texture.descriptor.id) {
                reserve_clock(&mut texture.animation, source.clock.as_ref());
            }
        }
    }
    pub fn apply(&self, material: &mut MObj, frame: f32) {
        replace_clock(&mut material.aobj, self.clock.as_ref(), frame);
        for texture in &mut material.textures {
            if let Some(source) = self.textures.iter().find(|t| t.id == texture.descriptor.id) {
                source.apply(texture, frame);
            }
        }
    }
}
impl PreparedTexture {
    fn apply(&self, texture: &mut TObj, frame: f32) {
        replace_clock(&mut texture.animation, self.clock.as_ref(), frame);
        texture.set_tables(Arc::clone(&self.images), Arc::clone(&self.palettes));
    }
}
fn reserve_clock(target: &mut Option<AObj>, source: Option<&AObj>) {
    if let Some(source) = source {
        let target = target.get_or_insert_with(AObj::default);
        let additional = source.fobj.len().saturating_sub(target.fobj.len());
        target.fobj.reserve(additional);
    }
}
fn replace_clock(target: &mut Option<AObj>, source: Option<&AObj>, frame: f32) {
    let Some(source) = source else {
        if let Some(target) = target {
            target.flags |= AOBJ_NO_ANIM;
            target.fobj.clear();
        }
        return;
    };
    let target = target
        .as_mut()
        .expect("material clock reserved before simulation");
    assert!(
        target.fobj.capacity() >= source.fobj.len(),
        "material tracks not reserved"
    );
    let mut tracks = std::mem::take(&mut target.fobj);
    tracks.clear();
    tracks.extend(source.fobj.iter().cloned());
    *target = AObj {
        flags: source.flags,
        curr_frame: source.curr_frame,
        rewind_frame: source.rewind_frame,
        end_frame: source.end_frame,
        framerate: source.framerate,
        fobj: tracks,
    };
    target.req_anim(frame);
}

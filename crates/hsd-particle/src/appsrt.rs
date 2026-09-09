//! Shared application SRT state (`psappsrt.c`, `psdisp.c`).
use crate::{system::ParticleSystem, Error};
use gekko_math::msl::sqrtf;
use hsd_anim::mtx::{hsd_mtx_srt, mtx_concat};
use hsd_types::{Mtx, Vec3};
use std::{collections::BTreeMap, sync::Arc};

/// `psAddGeneratorAppSRT` zeroes the allocation, then sets unit scale,
/// status, family ID and the first reference. Reference counts are derived
/// from live generator/particle owners, independently of Rust scratch clones.
#[derive(Debug, Clone, PartialEq)]
pub struct ApplicationTransform {
    pub translation: Vec3,
    pub rotation: Vec3,
    pub rotation_w: f32,
    pub scale: Vec3,
    pub status: i32,
    pub frame_number: u8,
    pub model_matrix: Mtx,
    /// AppSRT +0x64..0x90: camera view multiplied by the model matrix.
    pub model_view_matrix: Mtx,
    /// AppSRT +0x94/+0x98: lengths of the first two model-view columns.
    pub axis_scale: [f32; 2],
    pub family_id: u16,
    pub camera_facing: u8,
}

impl Default for ApplicationTransform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Vec3::ZERO,
            rotation_w: 0.0,
            scale: Vec3::new(1.0, 1.0, 1.0),
            status: 0,
            frame_number: 0,
            model_matrix: Mtx([[0.0; 4]; 3]),
            model_view_matrix: Mtx([[0.0; 4]; 3]),
            axis_scale: [0.0; 2],
            family_id: 0,
            camera_facing: 0,
        }
    }
}

impl ApplicationTransform {
    /// psDispSubAppSRT, inlined at 0x803A1F90..0x803A2184.
    /// The caller supplies the active camera and psFrameNum at display time.
    pub fn prepare_display(&mut self, view: &Mtx, frame_number: u8) -> Result<(), Error> {
        if self.frame_number == frame_number {
            return Ok(());
        }
        if self.camera_facing != 0 {
            return Err(Error::UnsupportedFeature(
                "psdisp.c:1426 camera-facing AppSRT",
            ));
        }
        if self.status != 2 {
            hsd_mtx_srt(
                &mut self.model_matrix,
                &self.scale,
                &self.rotation,
                &self.translation,
                None,
            );
        }
        if self.status == 1 {
            self.status = 2;
        }
        mtx_concat(view, &self.model_matrix, &mut self.model_view_matrix);
        for column in 0..2 {
            let [x, y, z] = self.model_view_matrix.0.map(|row| row[column]);
            // 803A1FFC..2014 and 803A2078..2090: three fmuls then
            // two fadds, no fusion. sqrtf's three Newton steps use
            // fnmsub at 803A202C/203C/204C (and the second column).
            self.axis_scale[column] = sqrtf(z * z + (x * x + y * y));
        }
        self.frame_number = frame_number;
        Ok(())
    }
}

impl ParticleSystem {
    /// Update only shared AppSRT caches visible in this display pass. The
    /// caller owns camera evaluation, display scheduling and psFrameNum.
    /// Particle simulation remains in local coordinates.
    pub fn prepare_application_transforms(
        &mut self,
        link_mask: u16,
        view: &Mtx,
        frame_number: u8,
    ) -> Result<(), Error> {
        let mut updated = BTreeMap::new();
        for (link, particles) in self.particles.iter().enumerate() {
            if link_mask & (1 << link) == 0 {
                continue;
            }
            for particle in particles {
                // psdisp.c:1896: particles smaller than FLT_EPSILON are culled.
                if particle.size < f32::EPSILON {
                    continue;
                }
                if let Some(id) = particle.appsrt_id {
                    if let std::collections::btree_map::Entry::Vacant(entry) = updated.entry(id) {
                        let mut transform = particle
                            .application_transform
                            .as_ref()
                            .expect("owned AppSRT")
                            .as_ref()
                            .clone();
                        transform.prepare_display(view, frame_number)?;
                        entry.insert(Arc::new(transform));
                    }
                }
            }
        }
        // Preserve shared ownership, including generators with no emitted
        // particle in this pass; scratch Arc clones never affect usedCount.
        for (id, transform) in self
            .generators
            .iter_mut()
            .map(|g| (g.appsrt_id, &mut g.application_transform))
            .chain(
                self.particles
                    .iter_mut()
                    .flatten()
                    .map(|p| (p.appsrt_id, &mut p.application_transform)),
            )
        {
            if let Some(replacement) = id.and_then(|id| updated.get(&id)) {
                *transform = Some(Arc::clone(replacement));
            }
        }
        Ok(())
    }
}

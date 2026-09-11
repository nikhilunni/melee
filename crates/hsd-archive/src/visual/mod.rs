//! Renderer-independent visual assets. Disc layouts stay in this crate;
//! no GPU handles, simulation state, or host-specific types are retained.
//!
//! Initial support decodes GX polygon display lists, skinning palettes, and
//! authored mip chains and material texture expressions. Shape animation remains
//! separate.
mod color;
mod light;
pub use light::{read_lights, Light};
mod material;
pub use material::PixelState;
mod particle;
mod polygon;
pub use particle::{read_particle_textures, ParticleTexture};
mod tev;
mod texture;
pub use polygon::{read_polygons, Influence, MatrixBinding, Polygon, Vertex};
pub use tev::{TevOperation, TextureCombiner};
pub use texture::{
    read_image, DecodedImages, MipLevel, Texture, TextureDecoder, TextureDescriptor, TextureLayer,
    TextureLod,
};

#[derive(Debug)]
pub enum VisualError {
    Archive(crate::Error),
    Invalid {
        offset: u32,
        reason: &'static str,
    },
    Unsupported {
        offset: u32,
        kind: &'static str,
        value: u32,
    },
}
impl From<crate::Error> for VisualError {
    fn from(error: crate::Error) -> Self {
        Self::Archive(error)
    }
}
impl std::fmt::Display for VisualError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Archive(e) => e.fmt(f),
            Self::Invalid { offset, reason } => write!(f, "visual data at {offset:#x}: {reason}"),
            Self::Unsupported {
                offset,
                kind,
                value,
            } => write!(f, "unsupported {kind} {value:#x} at {offset:#x}"),
        }
    }
}
impl std::error::Error for VisualError {}
type Result<T> = std::result::Result<T, VisualError>;
fn invalid(offset: u32, reason: &'static str) -> VisualError {
    VisualError::Invalid { offset, reason }
}
fn unsupported(offset: u32, kind: &'static str, value: u32) -> VisualError {
    VisualError::Unsupported {
        offset,
        kind,
        value,
    }
}

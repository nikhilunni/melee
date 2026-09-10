//! Shared Melee state types and enums: Fighter/Item/Stage structs, motion state ids, kinds. Leaf crate, no logic
//!
//! See CLAUDE.md for the porting rules that apply to every crate.
//!
//! Every enum here is a transcription of a C enum in the decomp. Each type's
//! doc comment cites the header it came from, and each variant's doc comment
//! carries the original C enumerator name so it can be grepped. Where the C
//! enum aliases one enumerator to another (Rust forbids duplicate
//! discriminants) or defines a count sentinel, the alias or sentinel is an
//! associated `const` on the enum instead of a variant.

/// A raw integer that does not name any variant of the target enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InvalidDiscriminant {
    /// Rust name of the enum the conversion targeted.
    pub type_name: &'static str,
    /// The rejected value.
    pub value: i32,
}

impl core::fmt::Display for InvalidDiscriminant {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} is not a valid {}", self.value, self.type_name)
    }
}

impl std::error::Error for InvalidDiscriminant {}

/// Declares a `#[repr(i32)]` enum transcribed from a C enum, together with
/// `From<Enum> for i32`, `TryFrom<i32> for Enum`, and an `ALL` slice listing
/// every variant in declaration order.
///
/// Every variant must carry an explicit literal discriminant so the value is
/// visible at the point of declaration and checkable against the header.
macro_rules! c_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident : $repr:ident {
            $(
                $(#[$vmeta:meta])*
                $variant:ident = $value:literal
            ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[repr($repr)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        $vis enum $name {
            $(
                $(#[$vmeta])*
                $variant = $value,
            )*
        }

        impl $name {
            /// Every variant, in header declaration order.
            pub const ALL: &'static [$name] = &[ $( $name::$variant, )* ];
        }

        impl From<$name> for $repr {
            #[inline]
            fn from(v: $name) -> $repr {
                v as $repr
            }
        }

        impl TryFrom<$repr> for $name {
            type Error = $crate::InvalidDiscriminant;

            fn try_from(v: $repr) -> Result<Self, Self::Error> {
                match v {
                    $( $value => Ok($name::$variant), )*
                    _ => Err($crate::InvalidDiscriminant {
                        type_name: stringify!($name),
                        value: v as i32,
                    }),
                }
            }
        }
    };
}

// The modules below are declared after `c_enum!` so the macro is in textual
// scope for them; no `use` is required.
mod cpu_cmd;
mod fighter_kind;
mod ft_part;
mod gr_kind;
mod ground_or_air;
mod item_kind;
mod motion_state;
mod player_kind;

pub use cpu_cmd::CpuCmd;
pub use fighter_kind::FighterKind;
pub use ft_part::FtPart;
pub use gr_kind::GrKind;
pub use ground_or_air::GroundOrAir;
pub use item_kind::ItemKind;
pub use motion_state::CommonMotionState;
pub use player_kind::PlayerKind;

#[cfg(test)]
mod tests {
    use super::*;

    /// Every variant survives `i32` and back, and no two variants share a
    /// value (the compiler enforces the latter, but the test documents it).
    fn round_trips_all<E>(all: &[E])
    where
        E: Copy
            + PartialEq
            + core::fmt::Debug
            + Into<i32>
            + TryFrom<i32, Error = InvalidDiscriminant>,
    {
        let mut seen = std::collections::HashSet::new();
        for &v in all {
            let raw: i32 = v.into();
            assert!(seen.insert(raw), "duplicate discriminant {raw} for {v:?}");
            assert_eq!(E::try_from(raw), Ok(v));
        }
    }

    #[test]
    fn fighter_kind() {
        // src/melee/ft/forward.h
        assert_eq!(i32::from(FighterKind::Mario), 0x00);
        assert_eq!(i32::from(FighterKind::Fox), 0x01);
        assert_eq!(i32::from(FighterKind::Popo), 0x0A);
        assert_eq!(i32::from(FighterKind::Nana), 0x0B);
        assert_eq!(i32::from(FighterKind::Purin), 0x0F);
        assert_eq!(i32::from(FighterKind::MasterH), 0x1B);
        assert_eq!(i32::from(FighterKind::CrezyH), 0x1C);
        assert_eq!(i32::from(FighterKind::Boy), 0x1D);
        assert_eq!(i32::from(FighterKind::Girl), 0x1E);
        assert_eq!(i32::from(FighterKind::GKoops), 0x1F);
        assert_eq!(i32::from(FighterKind::Sandbag), 0x20);
        assert_eq!(i32::from(FighterKind::None), 0x21);
        assert_eq!(FighterKind::MAX, i32::from(FighterKind::None));
        assert_eq!(FighterKind::try_from(2), Ok(FighterKind::Captain));
        assert_eq!(FighterKind::ALL.len(), 34);
        assert!(FighterKind::try_from(34).is_err());
        assert!(FighterKind::try_from(-1).is_err());
        round_trips_all(FighterKind::ALL);
    }

    #[test]
    fn ground_or_air() {
        // src/melee/ft/forward.h
        assert_eq!(i32::from(GroundOrAir::Ground), 0);
        assert_eq!(i32::from(GroundOrAir::Air), 1);
        assert_eq!(GroundOrAir::try_from(1), Ok(GroundOrAir::Air));
        assert!(GroundOrAir::try_from(2).is_err());
        round_trips_all(GroundOrAir::ALL);
    }

    #[test]
    fn common_motion_state() {
        // src/melee/ft/kinds/ftCommon/forward.h
        assert_eq!(i32::from(CommonMotionState::None), -1);
        assert_eq!(i32::from(CommonMotionState::DeadDown), 0);
        assert_eq!(i32::from(CommonMotionState::Wait), 14);
        assert_eq!(i32::from(CommonMotionState::Dash), 20);
        assert_eq!(i32::from(CommonMotionState::KneeBend), 24);
        assert_eq!(i32::from(CommonMotionState::Attack11), 44);
        assert_eq!(i32::from(CommonMotionState::DamageHi1), 75);
        assert_eq!(i32::from(CommonMotionState::GuardOn), 178);
        assert_eq!(i32::from(CommonMotionState::CliffCatch), 252);
        assert_eq!(i32::from(CommonMotionState::Barrel), 340);
        assert_eq!(CommonMotionState::COUNT, 341);
        assert_eq!(
            CommonMotionState::COUNT,
            i32::from(CommonMotionState::Barrel) + 1
        );
        assert_eq!(CommonMotionState::try_from(14), Ok(CommonMotionState::Wait));
        // 342 = 341 real states plus the -1 sentinel.
        assert_eq!(CommonMotionState::ALL.len(), 342);
        assert!(CommonMotionState::try_from(CommonMotionState::COUNT).is_err());
        assert!(CommonMotionState::try_from(-2).is_err());
        round_trips_all(CommonMotionState::ALL);
    }

    #[test]
    fn item_kind() {
        // src/melee/it/forward.h
        assert_eq!(i32::from(ItemKind::Capsule), 0x00);
        assert_eq!(i32::from(ItemKind::MBall), 0x22);
        assert_eq!(i32::from(ItemKind::LGunRay), 0x23);
        assert_eq!(i32::from(ItemKind::Kuriboh), 0x2B);
        assert_eq!(i32::from(ItemKind::MarioFire), 0x30);
        assert_eq!(i32::from(ItemKind::FoxLaser), 0x36);
        assert_eq!(i32::from(ItemKind::Coin), 0x9F);
        assert_eq!(i32::from(ItemKind::PokemonRandom), 0xA0);
        assert_eq!(i32::from(ItemKind::Tosakinto), 0xA1);
        assert_eq!(ItemKind::POKEMON_START, i32::from(ItemKind::Tosakinto));
        assert_eq!(i32::from(ItemKind::Fushigibana), 0xBE);
        assert_eq!(ItemKind::POKEMON_TERMINATE, 0xBF);
        assert_eq!(
            i32::from(ItemKind::ChicoritaLeaf),
            ItemKind::POKEMON_TERMINATE
        );
        assert_eq!(i32::from(ItemKind::PokemonUnk), 0xCF);
        assert_eq!(i32::from(ItemKind::OldKuri), 0xD0);
        assert_eq!(i32::from(ItemKind::Tincle), 0xDD);
        assert_eq!(i32::from(ItemKind::KyasarinEgg), 0xEC);
        assert_eq!(i32::from(ItemKind::None), -999);
        assert_eq!(ItemKind::try_from(-999), Ok(ItemKind::None));
        assert_eq!(ItemKind::try_from(0x10), Ok(ItemKind::LGun));
        // 0xED contiguous ids plus It_Kind_None.
        assert_eq!(ItemKind::ALL.len(), 0xED + 1);
        assert!(ItemKind::try_from(0xED).is_err());
        assert!(ItemKind::try_from(-1).is_err());
        round_trips_all(ItemKind::ALL);
    }

    #[test]
    fn gr_kind() {
        // src/melee/gr/forward.h
        assert_eq!(i32::from(GrKind::Unk00), 0x00);
        assert_eq!(i32::from(GrKind::Castle), 0x02);
        assert_eq!(i32::from(GrKind::Izumi), 0x0C);
        assert_eq!(i32::from(GrKind::PStadium), 0x10);
        assert_eq!(i32::from(GrKind::Battle), 0x24);
        assert_eq!(i32::from(GrKind::Last), 0x25);
        assert_eq!(i32::from(GrKind::Homerun), 0x43);
        assert_eq!(i32::from(GrKind::Figure3), 0x46);
        // The header writes an explicit 221 here, not 0x47.
        assert_eq!(GrKind::COUNT, 221);
        assert_eq!(GrKind::try_from(0x25), Ok(GrKind::Last));
        assert_eq!(GrKind::ALL.len(), 71);
        assert!(GrKind::try_from(0x47).is_err());
        assert!(GrKind::try_from(221).is_err());
        round_trips_all(GrKind::ALL);
    }

    #[test]
    fn player_kind() {
        // src/melee/pl/forward.h
        assert_eq!(i32::from(PlayerKind::Human), 0);
        assert_eq!(i32::from(PlayerKind::Cpu), 1);
        assert_eq!(i32::from(PlayerKind::Demo), 2);
        assert_eq!(i32::from(PlayerKind::Na), 3);
        assert_eq!(i32::from(PlayerKind::Boss), 4);
        assert_eq!(PlayerKind::try_from(1), Ok(PlayerKind::Cpu));
        assert!(PlayerKind::try_from(5).is_err());
        round_trips_all(PlayerKind::ALL);
    }

    #[test]
    fn cpu_cmd() {
        // src/melee/ft/ftcmdscript.h
        assert_eq!(i32::from(CpuCmd::PressA), 1);
        assert_eq!(i32::from(CpuCmd::ReleaseAll), 25);
        assert_eq!(i32::from(CpuCmd::Done), 0x7F);
        assert_eq!(CpuCmd::ZERO_ARG_END, 0x7F);
        assert_eq!(i32::from(CpuCmd::SetLstickX), 0x80);
        assert_eq!(i32::from(CpuCmd::PressAFor), 0x86);
        assert_eq!(i32::from(CpuCmd::WaitFor), 0x8E);
        assert_eq!(i32::from(CpuCmd::WaitIfMotionId), 0x92);
        assert_eq!(i32::from(CpuCmd::Unk0x93), 0x93);
        assert_eq!(i32::from(CpuCmd::LstickXTowardFighter), 0x95);
        assert_eq!(i32::from(CpuCmd::OneArgEnd), 0xBF);
        assert_eq!(CpuCmd::ONE_ARG_END, 0xBF);
        assert_eq!(i32::from(CpuCmd::LstickForwardClamped), 0xC2);
        assert_eq!(CpuCmd::COUNT, 0xC3);
        assert!(CpuCmd::COUNT <= i32::from(u8::MAX));
        assert_eq!(CpuCmd::ALL.len(), 52);
        // Opcode 0 is unused: the list starts at 1.
        assert!(CpuCmd::try_from(0).is_err());
        assert!(CpuCmd::try_from(0x96).is_err());
        assert!(CpuCmd::try_from(0xC3).is_err());
        // Byte conversions used by script readers.
        assert_eq!(u8::from(CpuCmd::Done), 0x7F);
        assert_eq!(CpuCmd::try_from(0x80u8), Ok(CpuCmd::SetLstickX));
        assert!(CpuCmd::try_from(0xFFu8).is_err());
        round_trips_all(CpuCmd::ALL);
    }

    #[test]
    fn invalid_discriminant_display() {
        let err = FighterKind::try_from(99).unwrap_err();
        assert_eq!(
            err,
            InvalidDiscriminant {
                type_name: "FighterKind",
                value: 99
            }
        );
        assert_eq!(err.to_string(), "99 is not a valid FighterKind");
    }
}

pub mod mp;
pub mod snapshot;

mod hit;
pub use hit::HitElement;

pub mod combat;
pub mod fixed;

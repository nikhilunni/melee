//! The Hylian shield: standing or crouching (ft_8008A348 / the SquatWait
//! entry's kind switch, ftLk_AttackAir_800EB3BC and ftCl_Init_8014919C) a
//! Link with his shield out (model group 2 at selection 0) raises a shield
//! volume (ftColl_8007B1B8 with the attribute block +C4) that stops
//! projectiles striking from in front (x221B_b3), only item hitboxes with
//! x42_b4 (x221B_b4), without bouncing them (x221B_b2). A stopped hit
//! pushes Link back (ftLk_800EB334 / ftCl_Init_80149114, shield_hit_cb).
use crate::LinkFamily;
use melee_ft::fighter::{assets::FighterAssets, damage::DefenseVolume, Fighter};

/// The volume while it is up, and a hit it stopped this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HylianShield {
    /// x221B_b0 with this callback: raised by the last Wait/SquatWait entry
    /// (every motion change lowers it).
    pub raised: bool,
    /// x19A4 and specialn_facing_dir (ftColl_80077688's fighter half).
    pub pending: Option<(i32, f32)>,
}

/// ftLk_AttackAir_800EB3BC (800EB3BC) / ftCl_Init_8014919C: with the shield
/// in hand (x5F4_arr[2].prev == 0), raise the volume.
pub fn raise<C: LinkFamily>(f: &mut Fighter) {
    if f.character.get::<C>().shield_model_group() != 0 {
        return;
    }
    f.character.get_mut::<C>().specials().hylian.raised = true;
}

/// x221B's b0 (installed) with the Hylian shield's b2, b3 and b4.
const RAISED_FLAGS: u8 = 0x80 | 0x20 | 0x10 | 0x08;

/// A savestate's Fighter bytes: the volume is up when x221B carries the
/// shield's flags (the Wait it was raised in began before the save).
pub fn restore_saved(hylian: &mut HylianShield, raw_fighter: &[u8]) {
    hylian.raised = raw_fighter[0x221B] & RAISED_FLAGS == RAISED_FLAGS;
}

fn volume<C: LinkFamily>(f: &mut Fighter) -> DefenseVolume {
    let s = f.character.get::<C>().attributes().shield.clone();
    let bone = usize::try_from(s.bone).expect("Hylian shield part");
    let c = &mut f.core;
    let position =
        melee_ft::fighter::caches::bone_position(&mut c.skeleton, c.animation.root, bone, s.offset);
    let matrix = *c.skeleton.get_mtx(c.animation.parts[bone].joint);
    DefenseVolume {
        position,
        matrix,
        radius: s.radius,
        fixed_bounce: false,
        no_bounce: true,
    }
}

/// ftColl_8007925C's shield step against the Hylian shield (ftcoll.c:2239-
/// 2282): not inert, shieldable (x42_b1), from the front, x42_b4, touching
/// the volume (lbColl_80007BCC); then ftColl_80077688.
pub fn item_contact<C: LinkFamily>(
    f: &mut Fighter,
    item: &mut melee_it::ItemCore,
    id: usize,
    assets: &FighterAssets,
) -> bool {
    if !f.character.get::<C>().specials_ref().hylian.raised {
        return false;
    }
    let x = item.position.x;
    let in_front = if f.physics.facing == -1.0 {
        f.physics.position.x >= x
    } else {
        f.physics.position.x <= x
    };
    if !in_front || !item.hit_flags[id].defense_interaction {
        return false;
    }
    let volume = volume::<C>(f);
    let hit = item.hitboxes[id].clone().expect("eligible item hit");
    use melee_coll::geometry::{capsule_contact, Capsule};
    let Some(contact) = capsule_contact(
        Capsule {
            start: hit.previous_position,
            end: hit.position,
            radius: hit.descriptor.radius
                * if hit.descriptor.ignore_scale {
                    1.0
                } else {
                    item.scale
                },
        },
        Capsule {
            start: volume.position,
            end: volume.position,
            radius: volume.radius,
        },
        &volume.matrix,
        20.0 * f.core.player.scale,
    ) else {
        return false;
    };
    // shield_unk1 is zero outside the counters: no x1964, no item xCC0.
    let damage = f.record_item_volume_hit(item, id, contact, &volume, 0.0, assets);
    let facing = if f.physics.position.x > item.position.x {
        -1.0
    } else {
        1.0
    };
    let hylian = &mut f.character.get_mut::<C>().specials().hylian;
    if hylian.pending.is_none_or(|(old, _)| damage > old) {
        hylian.pending = Some((damage, facing));
    }
    true
}

/// Fighter_ProcessHit's x19A4 branch (fighter.c:2909-2918): the
/// shield_hit_cb pushes Link back from where the hit came
/// (ftCo_80092ED8 of x19A4 and the attribute +D8, times PlCo +294), and
/// x19A4 sets the hitlag.
pub fn process_hit<C: LinkFamily>(f: &mut Fighter, assets: &FighterAssets) {
    let Some((damage, facing)) = f.character.get_mut::<C>().specials().hylian.pending.take() else {
        return;
    };
    let scale = f.character.get::<C>().attributes().shield.damage_scale;
    let p = &assets.shield;
    use gekko_math::fma::fmadds;
    // retail 80092F08 / 80092F20: fmadds, a separate subtract and product.
    let light = fmadds(
        scale,
        p.stun_lightshield[1] - p.stun_lightshield[0],
        p.stun_lightshield[0],
    );
    let stun = fmadds(
        p.stun_multiplier,
        damage as f32 * (1.0 - light),
        p.stun_base,
    );
    // ftLk_800EB334: fmuls, negated unless the hit came from the left.
    let push = stun * p.pushback_multiplier;
    f.physics.ground_velocity = if facing < 0.0 { push } else { -push };
    f.combat.dealt_damage = damage;
}

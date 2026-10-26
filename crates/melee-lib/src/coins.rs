//! A Coin match's purses (gm_8016B0B4): StaticPlayer +90..+9C, by slot.

/// Player slots the purses hold.
const SLOTS: usize = 6;
/// StaticPlayer's first purse word and stride.
const STATIC_PLAYER: u32 = 0x8045_3080;
const STATIC_PLAYER_STRIDE: u32 = 0xE90;
const COINS_OFFSET: u32 = 0x90;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Purse {
    /// +90, Player_GetCoins: the coins the player holds, its score.
    pub current: i32,
    /// +94, Player_GetTotalCoins: every coin it picked up.
    pub total: i32,
    /// +98, Player_GetUnk98: coins its hits knocked out of others
    /// (ftCommon_8007EA90).
    pub knocked_out: i32,
    /// +9C, Player_GetUnk9C: coins it lost falling (ftCo_800D4E50).
    pub lost: i32,
}

/// Every slot's purse.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Purses(pub [Purse; SLOTS]);

impl Purses {
    /// The purses a savestate holds.
    pub(crate) fn saved(bytes: impl Fn(u32, usize) -> Vec<u8>) -> Self {
        Self(std::array::from_fn(|slot| {
            let raw = bytes(
                STATIC_PLAYER + slot as u32 * STATIC_PLAYER_STRIDE + COINS_OFFSET,
                16,
            );
            let word = |at: usize| i32::from_be_bytes(raw[at..at + 4].try_into().unwrap());
            Purse {
                current: word(0),
                total: word(4),
                knocked_out: word(8),
                lost: word(12),
            }
        }))
    }

    fn slot(&mut self, slot: u8) -> &mut Purse {
        &mut self.0[usize::from(slot)]
    }

    /// ftCommon_8007EA90's Player_SetUnk98: a hit's spill is credited to
    /// its striker.
    pub(crate) fn credit_knockout(&mut self, striker: Option<u8>, total: i32) {
        if let Some(striker) = striker {
            let purse = self.slot(striker);
            purse.knocked_out = purse.knocked_out.wrapping_add(total);
        }
    }

    /// ftCo_800D4E50's purse: half the coins (rounded toward zero) leave
    /// it, counted as lost; at most `limit` (PlCo +5C8) of them spill.
    pub(crate) fn take_fall(&mut self, slot: u8, limit: i32) -> i32 {
        let purse = self.slot(slot);
        // 800D4EA8: srawi, addze.
        let taken = purse.current / 2;
        purse.current -= taken;
        purse.lost = purse.lost.wrapping_add(taken);
        taken.min(limit)
    }

    /// ft_8007C77C's take: a coin's worth into the purse and its total.
    pub(crate) fn take_coin(&mut self, slot: u8, value: i32) {
        let purse = self.slot(slot);
        purse.current = purse.current.wrapping_add(value);
        purse.total = purse.total.wrapping_add(value);
    }
}

/// gm_8016B0B4 at the match's start: every fighter of a Coin match spills
/// ItCo's coins and takes them with its spheres (placed here for a cold
/// start, whose fighters were created before the rules; a savestate's
/// fighters carry their own).
pub(crate) fn start(state: &mut crate::initial_state::InitialState, cold: bool) {
    if state.clock.scoring.match_kind != melee_types::MatchKind::Coin {
        return;
    }
    if state.item_spawner.is_some() {
        unimplemented!(
            "it_8026F8B4 (itdrop.c:192): a Coin match's containers drop coins \
             (it_802E5F00); no witness has items on"
        );
    }
    let coin = state.assets.items.get(melee_types::ItemKind::Unk4);
    let attributes = melee_it::coin::CoinAttributes::read(&coin.special_attributes);
    for (index, fighter) in state.fighters.iter_mut().enumerate() {
        crate::scene_fighter::with_fighter!(fighter, |f| {
            f.core.coin_rules = Some(melee_ft::fighter::coins::CoinRules { attributes });
            if cold {
                f.core.place_coin_spheres(&state.assets.fighters[index]);
            }
        });
    }
}

/// it_802E6658 (802E6658), before each coin: once the coins
/// (Item_804A0C64 x50) reach two short of ItCo's limit, the last coin in
/// the item list (it_80272828) is destroyed at once (Item_8026A8EC).
fn make_room(items: &mut melee_it::ItemPool) {
    let limit = items.common().hold_limits[usize::from(melee_it::coin::HOLD_KIND)]
        .expect("ItCo's coin limit");
    let live = |item: &&mut melee_it::ItemCore| item.kind == melee_types::ItemKind::Unk4 && !item.destroyed;
    // 802E6680: cmplw.
    if (items.iter_mut().filter(live).count() as u32) < (limit as u32).wrapping_sub(2) {
        return;
    }
    // ItCo's limit is 60: random two-player play never holds 58 coins at
    // once (each lives 280 frames), so the removal has no witness yet.
    unimplemented!("it_802E6658 (it_2E5A.c): a spill with 58 coins out removes the last one");
}

/// ft_8007C77C (8007C77C), in Fighter_8006CB94 after the fighters' hits:
/// on the frames whose parity (gm_801A4BB8) is the player's, a fighter not
/// intangible (ftColl_8007B868) takes every coin open to it (its own after
/// x14, others' after x10) that one of its two spheres swept past
/// (lbColl_80007B78): the worth into its purse, sound 0x93, effect 0x432
/// where the coin was met, and the coin destroyed at once (Item_8026A8EC).
pub(crate) fn pick_up(
    state: PickupScene<'_>,
    player: usize,
    fighter: &mut crate::scene_fighter::SceneFighter,
) -> anyhow::Result<()> {
    let PickupScene {
        items,
        purses,
        scene_frame,
        effects,
        particles,
        rng,
        assets,
    } = state;
    crate::scene_fighter::with_fighter!(fighter, |f| {
        if f.core.coin_rules.is_none() {
            return Ok(());
        }
        // 8007C7A4..C0: gm_801A4BB8() & 1 against player_id & 1 (x2229_b4).
        if scene_frame & 1 != u32::from(f.player.id) & 1 {
            return Ok(());
        }
        // ftColl_8007B868 == 2: x1988 or x198C intangible.
        if f.commands.hurt_status == melee_types::combat::HurtStatus::Intangible
            || f.status.ledge_intangibility != 0
        {
            return Ok(());
        }
        let slot = f.player.id;
        let scale = f.player.scale;
        let spheres = f.core.coin_spheres;
        for index in 0..items.len() {
            let item = items.common_and_item_mut(index).1;
            if item.kind != melee_types::ItemKind::Unk4 || item.destroyed {
                continue;
            }
            let coin = *item.coin();
            // ftLib_80086FD4: the owner is this player.
            let open = if item.owner == Some(slot) {
                coin.open_to_owner
            } else {
                coin.open_to_others
            };
            if !open {
                continue;
            }
            for sphere in &spheres {
                // lbColl_80007B78: the fighter's radius times x34_scale.y,
                // the coin's times its scl (fmuls each).
                let Some(contact) = melee_coll::sweep::swept_circles_touch(
                    sphere.previous_center,
                    sphere.center,
                    coin.previous_center,
                    coin.center,
                    sphere.radius * scale,
                    coin.radius * item.scale,
                ) else {
                    continue;
                };
                purses.take_coin(slot, coin.value);
                // lbAudioAx_80023870(0x93, 0x7F, 0x40, 0x1A): no simulated
                // listener. efSync_Spawn(0x432, NULL, &sub+0x20, &1.0f).
                f.core
                    .effects
                    .push(melee_ef::request::EffectRequest::PositionalGenerator {
                        id: 0x432,
                        position: contact.second,
                    });
                effects.flush::<melee_ft::fighter::RetailTrig>(
                    melee_ef::EffectTiming::Immediate,
                    player,
                    &mut f.core,
                    &assets.common_particle_bank,
                    &assets.particle_bank,
                    particles,
                    rng,
                )?;
                item.destroyed = true;
                // Item_8026A8EC frees the coin here, ahead of what the rest
                // of the proc creates; its efLib_DestroyAll walks the
                // generators (hsd_8039D688), leaving the insertion cursor
                // at the list's tail before the next coin's effect.
                particles.walk_unowned_joint();
                effects.gobjs.mark();
                items.note_destroyed();
                break;
            }
        }
        Ok(())
    })
}

/// What ft_8007C77C touches besides the fighter.
pub(crate) struct PickupScene<'a> {
    pub items: &'a mut melee_it::ItemPool,
    pub purses: &'a mut Purses,
    pub scene_frame: u32,
    pub effects: &'a mut melee_ef::Effects,
    pub particles: &'a mut hsd_particle::system::ParticleSystem,
    pub rng: &'a mut gekko_math::HsdRng,
    pub assets: &'a crate::assets::Assets,
}

/// The spilling fighter (it_802E614C's parent GObjs) and the proc it
/// spills in.
#[derive(Clone, Copy)]
pub(crate) struct SpillOwner<'a> {
    pub slot: u8,
    pub secondary: bool,
    pub after_hitbox_refresh: bool,
    pub efasync_immediate: bool,
    pub bounds: &'a melee_it::ItemBounds,
}

/// it_802E609C's loop for a planned spill: each coin made in turn
/// (Item_80268B18, then it_802E6380, it_802E66A0 and it_802750F8 through
/// the kind's set-up), after it_802E6658 made room.
#[allow(clippy::too_many_arguments)]
pub(crate) fn create_spill(
    items: &mut melee_it::ItemPool,
    gobjs: &mut hsd_gobj::GObjAddresses,
    resources: &crate::scene_items::Resources,
    map: &mut melee_mp::CollMap,
    world: &mut hsd_gobj::TaggedWorld,
    objects: &mut crate::scene_items::Objects,
    owner: SpillOwner<'_>,
    position: hsd_types::Vec3,
    spill: melee_it::coin::Spill,
    coins: &melee_types::fixed::FixedVec<melee_it::coin::PlannedCoin, { melee_it::coin::MAX_SPILL }>,
    rng: &mut gekko_math::HsdRng,
) {
    for coin in coins.iter() {
        make_room(items);
        let spawn = melee_it::coin::coin_spawn(Some(owner.slot), position, spill, coin);
        crate::scene_items::request(
            items,
            gobjs,
            resources,
            map,
            world,
            objects,
            melee_it::ItemRequest::Spawn(spawn),
            crate::scene_items::RequestOwner {
                slot: Some(owner.slot),
                secondary: owner.secondary,
                held_item: None,
                after_hitbox_refresh: owner.after_hitbox_refresh,
                efasync_immediate: owner.efasync_immediate,
                stale_multiplier: 1.0,
                bounds: Some(owner.bounds),
            },
            rng,
        )
        .expect("Item_80268B18: a coin under ItCo's limit");
        items.note_destroyed();
    }
}

//! Mr. Game & Watch's hold on his articles: the fp->u.gw GObj references,
//! the accessory4 callbacks that create them, ftGw_Init_OnDamage (which
//! removes them all) and the owner side of their hitlag pairs.
use crate::init::GameWatch;
use hsd_types::Vec3;
use melee_ft::fighter::Fighter;
use melee_it::{ItemControl, ItemRequest, SpawnItem};
use melee_types::{FtPart, GroundOrAir, ItemKind};

/// fp->u.gw.x2248..x226C: which articles are out.
#[derive(Clone, Copy, Debug, Default)]
pub struct Articles {
    /// x224C_greenhouseGObj.
    pub greenhouse: bool,
    /// x2250_manholeGObj2.
    pub manhole: bool,
    /// x2254_fireGObj.
    pub torch: bool,
    /// x2258_parachuteGObj.
    pub parachute: bool,
    /// x225C_turtleGObj.
    pub turtle: bool,
    /// x2260_sparkyGObj.
    pub sparky: bool,
    /// death2_cb and take_dmg_cb are ftGw_Init_OnDamage until the next
    /// motion change.
    pub damage_callbacks: bool,
}

/// accessory4_cb while one of the character rows owns it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Accessory {
    #[default]
    None,
    /// ftGw_Attack11_ItemGreenhouseSetup.
    GreenhouseSetup,
    /// ftGw_Attack11_DecideAction.
    GreenhouseMotion,
    /// ftGw_AttackLw3_ItemManholeSetup.
    ManholeSetup,
    /// ftGw_ItemTorchSetup.
    TorchSetup,
    /// ftGw_AttackAirN_ItemParachuteSetup / ItemTurtleSetup / ItemSparkySetup.
    AerialSetup(Aerial),
    /// ftGw_AttackAirN_ItemParachuteOnLand / ItemTurtleOnLand /
    /// ItemSparkyOnLand.
    AerialLanded(Aerial),
}

/// The three aerials with an article.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aerial {
    Neutral,
    Back,
    Up,
}
impl Aerial {
    pub const ALL: [Self; 3] = [Self::Neutral, Self::Back, Self::Up];
    pub fn kind(self) -> ItemKind {
        match self {
            Self::Neutral => ItemKind::GameWatchParachute,
            Self::Back => ItemKind::GameWatchTurtle,
            Self::Up => ItemKind::GameWatchBreath,
        }
    }
    /// The part each article hangs from (ftgamewatchattackair.c:36, :97,
    /// :168: fp->parts indexed by the enum's own value).
    fn part(self) -> FtPart {
        match self {
            Self::Neutral => FtPart::TransN,
            Self::Back => FtPart::LShoulderN,
            Self::Up => FtPart::LHandN,
        }
    }
    fn out(self, articles: &mut Articles) -> &mut bool {
        match self {
            Self::Neutral => &mut articles.parachute,
            Self::Back => &mut articles.turtle,
            Self::Up => &mut articles.sparky,
        }
    }
}

fn articles(f: &mut Fighter) -> &mut Articles {
    &mut f.character.get_mut::<GameWatch>().articles
}

/// `fp->accessory4_cb = callback` after a motion change.
pub fn install(f: &mut Fighter, accessory: Accessory) {
    f.character.get_mut::<GameWatch>().accessory = accessory;
    f.core.arm_accessory4();
}

/// `fp->accessory4_cb = NULL`.
fn uninstall(f: &mut Fighter) {
    f.character.get_mut::<GameWatch>().accessory = Accessory::None;
    f.core.accessory4_armed = false;
}

/// Item_InitSpawn at the part's world position (lb_8000B1CC), Item_80268B18,
/// then Item_AttachGameWatchArticle: the article hangs from fp->parts[part].
fn spawn(f: &mut Fighter, kind: ItemKind, part: FtPart) {
    let part = i32::from(part) as u8;
    let c = &mut f.core;
    let position = melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        usize::from(part),
        Vec3::ZERO,
    );
    let spawn = SpawnItem::attached(kind, c.player.id, position, c.physics.facing);
    c.item_requests.push(ItemRequest::SpawnInHand {
        spawn,
        part,
        hold: false,
        catch_item: false,
        scale_by_owner: false,
    });
}

fn control(f: &mut Fighter, kind: ItemKind, control: ItemControl) {
    let owner = f.player.id;
    f.core.item_requests.push(ItemRequest::Control {
        owner,
        kind,
        control,
    });
}

/// death2_cb / take_dmg_cb = ftGw_Init_OnDamage, and the article's hitlag
/// pair (it_8026B724 / it_8026B73C through the kind's wrappers).
fn install_callbacks(f: &mut Fighter, kind: ItemKind) {
    articles(f).damage_callbacks = true;
    f.effect_state.article_hitlag = Some(kind);
}

/// Fighter_8006C80C: the installed accessory4.
pub fn accessory(f: &mut Fighter) {
    if !f.core.accessory4_armed {
        return;
    }
    match f.character.get::<GameWatch>().accessory {
        Accessory::None => {}
        Accessory::GreenhouseSetup => greenhouse_setup(f),
        Accessory::GreenhouseMotion => greenhouse_motion(f),
        Accessory::ManholeSetup => manhole_setup(f),
        Accessory::TorchSetup => torch_setup(f),
        Accessory::AerialSetup(aerial) => aerial_setup(f, aerial),
        Accessory::AerialLanded(aerial) => aerial_landed(f, aerial),
    }
}

/// ftGw_Attack11_ItemGreenhouseSetup (8014BDB0): the sprayer on the left
/// hand, or its motion when it is already out.
fn greenhouse_setup(f: &mut Fighter) {
    if articles(f).greenhouse {
        greenhouse_motion(f);
    } else {
        spawn(f, ItemKind::GameWatchGreenhouse, FtPart::LHandNb);
        articles(f).greenhouse = true;
        install_callbacks(f, ItemKind::GameWatchGreenhouse);
    }
    uninstall(f);
}

/// ftGw_Attack11_DecideAction (8014BE84): the sprayer takes the motion of
/// its owner's jab row (itGamewatchGreenhouse_802C6430..802C64A8). Without
/// a sprayer the accessory stays installed.
fn greenhouse_motion(f: &mut Fighter) {
    if !articles(f).greenhouse {
        return;
    }
    let motion = f.motion_state.action.0;
    if (crate::attack::ATTACK_11.0..=crate::attack::ATTACK_100_END.0).contains(&motion) {
        control(
            f,
            ItemKind::GameWatchGreenhouse,
            ItemControl::Motion(motion - crate::attack::ATTACK_11.0),
        );
    }
    install_callbacks(f, ItemKind::GameWatchGreenhouse);
    uninstall(f);
}

/// ftGw_AttackLw3_ItemManholeSetup (8014AB48): the Manhole on the left
/// hand. A held item is put away under it (x2248_manholeGObj).
fn manhole_setup(f: &mut Fighter) {
    if !articles(f).manhole {
        if f.core.held_item.is_some() {
            unimplemented!(
                "ftGw_AttackLw3_ItemManholeSetup (ftgamewatchattacklw3.c:36-41): a held item stowed under the Manhole"
            );
        }
        spawn(f, ItemKind::GameWatchManhole, FtPart::LHandNb);
        articles(f).manhole = true;
    }
    articles(f).damage_callbacks = true;
    f.effect_state.article_hitlag = Some(ItemKind::GameWatchManhole);
    uninstall(f);
}

/// ftGw_ItemTorchSetup (8014A848): the torch on the left hand.
fn torch_setup(f: &mut Fighter) {
    spawn(f, ItemKind::GameWatchFire, FtPart::LHandNb);
    articles(f).torch = true;
    install_callbacks(f, ItemKind::GameWatchFire);
    uninstall(f);
}

/// ftGw_AttackAirN_ItemParachuteSetup (8014AFC0), ItemTurtleSetup
/// (8014B1B4), ItemSparkySetup (8014B3A8): the aerial's article on its
/// part, or the landed callback when it is already out.
fn aerial_setup(f: &mut Fighter, aerial: Aerial) {
    if *aerial.out(articles(f)) {
        aerial_landed(f, aerial);
    } else {
        spawn(f, aerial.kind(), aerial.part());
        *aerial.out(articles(f)) = true;
        install_callbacks(f, aerial.kind());
    }
    uninstall(f);
}

/// ftGw_AttackAirN_ItemParachuteOnLand (8014B074), ItemTurtleOnLand
/// (8014B268), ItemSparkyOnLand (8014B45C): the aerial articles thaw; in
/// LandingAirN the article takes its landed motion (all three test that
/// row, so only the parachute ever does). Without the article the
/// accessory stays installed.
fn aerial_landed(f: &mut Fighter, aerial: Aerial) {
    if !*aerial.out(articles(f)) {
        return;
    }
    thaw_aerial_articles(f);
    if f.motion_state.action == crate::attack_air::LANDING_AIR_N {
        control(
            f,
            aerial.kind(),
            ItemControl::Motion(crate::attack_air::ARTICLE_LANDED),
        );
    }
    install_callbacks(f, aerial.kind());
    uninstall(f);
}

/// ftGw_AttackAirN_ExitItemHitlag (8014B5CC): it_8026B73C on each aerial
/// article that is out.
pub fn thaw_aerial_articles(f: &mut Fighter) {
    set_aerial_articles_frozen(f, false, None);
}

fn set_aerial_articles_frozen(f: &mut Fighter, frozen: bool, except: Option<ItemKind>) {
    let out = *articles(f);
    for (present, kind) in [
        (out.parachute, ItemKind::GameWatchParachute),
        (out.turtle, ItemKind::GameWatchTurtle),
        (out.sparky, ItemKind::GameWatchBreath),
    ] {
        if present && Some(kind) != except {
            control(f, kind, ItemControl::OwnerHitlag(frozen));
        }
    }
}

fn is_aerial(kind: ItemKind) -> bool {
    matches!(
        kind,
        ItemKind::GameWatchParachute | ItemKind::GameWatchTurtle | ItemKind::GameWatchBreath
    )
}

/// ftGw_AttackAirN_EnterItemHitlag (8014B574) beyond the article the scene
/// froze: the other aerial articles that are out.
pub fn hitlag_begin(f: &mut Fighter) {
    let frozen = f.effect_state.article_hitlag;
    if frozen.is_some_and(is_aerial) {
        set_aerial_articles_frozen(f, true, frozen);
    }
}

/// ftGw_AttackAirN_ExitItemHitlag (8014B5CC) beyond the article the scene
/// thawed. The scene calls this after the motion's hitlag pair may have
/// gone, so every aerial article that is out thaws.
pub fn hitlag_end(f: &mut Fighter) {
    thaw_aerial_articles(f);
}

/// The Destroyed callbacks: each article lets go of its owner.
pub fn destroyed(f: &mut Fighter, kind: ItemKind) {
    let a = articles(f);
    match kind {
        // ftGw_Attack11_ItemGreenhouseSetFlag (8014BF48).
        ItemKind::GameWatchGreenhouse => a.greenhouse = false,
        // ftGw_AttackLw3_ItemManholeRemove (8014AC40); the stowed item's
        // return fails closed at the stow.
        ItemKind::GameWatchManhole => a.manhole = false,
        // ftGw_AttackS4_ItemTorchSetFlag (8014A904).
        ItemKind::GameWatchFire => a.torch = false,
        // ftGw_AttackAirN_ItemParachuteSetFlag (8014B0F0), ItemTurtleSetFlag
        // (8014B2E4), ItemSparkySetFlag (8014B4D8): death2_cb and
        // take_dmg_cb go with the article.
        ItemKind::GameWatchParachute => {
            a.parachute = false;
            a.damage_callbacks = false;
        }
        ItemKind::GameWatchTurtle => {
            a.turtle = false;
            a.damage_callbacks = false;
        }
        ItemKind::GameWatchBreath => {
            a.sparky = false;
            a.damage_callbacks = false;
        }
        _ => {}
    }
}

/// ftGw_Init_OnDamage (8014A4CC): every article that is out is destroyed
/// (Item_8026A8EC through the kind's remover) and lets go of its owner.
pub fn on_damage(f: &mut Fighter) {
    let out = *articles(f);
    for (present, kind) in [
        (out.greenhouse, ItemKind::GameWatchGreenhouse),
        (out.manhole, ItemKind::GameWatchManhole),
        (out.torch, ItemKind::GameWatchFire),
        (out.parachute, ItemKind::GameWatchParachute),
        (out.turtle, ItemKind::GameWatchTurtle),
        (out.sparky, ItemKind::GameWatchBreath),
    ] {
        if present {
            control(f, kind, ItemControl::Remove);
            destroyed(f, kind);
        }
    }
    crate::special_s::remove_judgement(f);
    crate::special_lw::remove_panic(f);
    crate::special_hi::remove_rescue(f);
}

/// ftCommon_8007DB58 / ftCo_800D331C: take_dmg_cb and death2_cb while a
/// motion installed them.
pub fn damage_callback(f: &mut Fighter) {
    if articles(f).damage_callbacks {
        on_damage(f);
    }
}

/// ftGw_Init_8014A538 (8014A538): a grounded attack that left the floor
/// puts every article away.
pub fn airborne_cleanup(f: &mut Fighter) {
    if f.physics.ground_or_air == GroundOrAir::Air {
        on_damage(f);
    }
}

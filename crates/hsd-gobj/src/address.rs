//! Which address a new GObj takes: the allocator's free list.
//!
//! Every GObj comes from one object allocator (`gobj_alloc_data`,
//! gobjplink.c:29-34). HSD_ObjAlloc (0x8037ABC8) pops the head of its free
//! list and HSD_ObjFree (0x8037AD20) pushes the freed object there
//! (objalloc.c:109-126), so a new object takes the address of the object
//! freed last, whatever its kind: an item can sit where an effect model or a
//! screen-shake model was.
//!
//! Melee keeps GObj pointers past the object's life. A hitbox's victim list
//! (lbColl_80008688) still names an item that has been destroyed, and a
//! later item created at that address is taken for a victim the hitbox has
//! already struck. The port has no addresses; it keeps the identity they
//! give: a [`GObjAddress`] is equal to another exactly when retail's two
//! pointers would be.
//!
//! The order of every creation and destruction in a match decides those
//! identities, so each one is recorded here in retail's order, by every
//! subsystem that owns GObjs. `harness/dolphin/gobj_ledger.py` records the
//! same sequence from retail; `melee-sim gobj-diff` compares the two.

/// The most addresses free at once: every object of a match that can be
/// destroyed during it (items, effect models, screen-shake models, HUD
/// banners) could be free together.
const CAPACITY: usize = 512;
/// The most creations and destructions one tick's diagnostic log keeps.
const LOG_CAPACITY: usize = 192;

/// A GObj's address, as an identity: equal for two objects exactly when
/// retail created the second where the first had been.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct GObjAddress(u32);

impl GObjAddress {
    /// The identity as a number, for a list that keys several kinds of
    /// victim in one word.
    pub fn index(self) -> u32 {
        self.0
    }
}

/// What an object is, as retail's GObj says it: the classifier and p_link
/// GObj_Create is given. Only the diagnostic log reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GObjClass {
    /// Classifier 2, p_link 4: a stage's enemy generator
    /// (grZakoGenerator_801CAE04).
    EnemyGenerator,
    /// HSD_GOBJ_CLASS_STAGE (3), p_link 5: a stage proc or map object
    /// (ground.c).
    Stage,
    /// HSD_GOBJ_CLASS_STAGE (3), p_link 18: a screen-shake model
    /// (grLib_801C9CEC).
    Quake,
    /// HSD_GOBJ_CLASS_ITEM (6), p_link 9 (Item_80268B18, item.c:957).
    Item,
    /// HSD_GOBJ_CLASS_ITEMLINK (7), p_link 10: a chain's or tether's link
    /// object.
    ItemLink,
    /// HSD_GOBJ_CLASS_EFFECT (8), p_link 11 or 12: an effect model
    /// (efLib_Create, eflib.c:481).
    Effect,
    /// HSD_GOBJ_CLASS_UI (0xE): a HUD model.
    Hud,
}

impl GObjClass {
    /// The GObj's classifier (gobj.h).
    pub fn classifier(self) -> u16 {
        match self {
            Self::EnemyGenerator => 2,
            Self::Stage | Self::Quake => 3,
            Self::Item => 6,
            Self::ItemLink => 7,
            Self::Effect => 8,
            Self::Hud => 0xE,
        }
    }
}

/// One creation or destruction, for the diagnostic log.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GObjEvent {
    pub created: bool,
    pub class: GObjClass,
    pub address: GObjAddress,
}

/// `gobj_alloc_data`'s free list, from the first tick of a match (the scene
/// initialises the allocator, and what it creates before the first tick is
/// taken from an empty list).
#[derive(Clone, Debug)]
pub struct GObjAddresses {
    /// The free list, head last.
    free: [GObjAddress; CAPACITY],
    free_len: usize,
    /// The next address the heap would give (HSD_ObjAllocAddFree).
    fresh: u32,
    /// This tick's events, for diagnostics; not game state.
    log: [Option<GObjEvent>; LOG_CAPACITY],
    log_len: usize,
}

impl Default for GObjAddresses {
    fn default() -> Self {
        Self {
            free: [GObjAddress(0); CAPACITY],
            free_len: 0,
            fresh: 1,
            log: [None; LOG_CAPACITY],
            log_len: 0,
        }
    }
}

impl GObjAddresses {
    /// HSD_ObjAlloc (0x8037ABC8) from GObj_Create: the free list's head, or
    /// new memory when the list is empty (objalloc.c:103-110).
    pub fn create(&mut self, class: GObjClass) -> GObjAddress {
        let address = if self.free_len == 0 {
            self.outside()
        } else {
            self.free_len -= 1;
            self.free[self.free_len]
        };
        self.record(true, class, address);
        address
    }

    /// HSD_ObjFree (0x8037AD20) from HSD_GObjPLink_80390228: the object
    /// becomes the free list's head.
    pub fn destroy(&mut self, class: GObjClass, address: GObjAddress) {
        assert!(self.free_len < CAPACITY, "GObj free list bound exceeded");
        self.free[self.free_len] = address;
        self.free_len += 1;
        self.record(false, class, address);
    }

    /// The address of an object created before the match's first tick (a
    /// HUD model, say), which no object of the match has had; for its
    /// [`destroy`](Self::destroy) during the match.
    pub fn outside(&mut self) -> GObjAddress {
        let address = GObjAddress(self.fresh);
        self.fresh += 1;
        address
    }

    fn record(&mut self, created: bool, class: GObjClass, address: GObjAddress) {
        // A tick with more events than the log holds keeps the first ones.
        if self.log_len < LOG_CAPACITY {
            self.log[self.log_len] = Some(GObjEvent {
                created,
                class,
                address,
            });
            self.log_len += 1;
        }
    }

    /// The creations and destructions since the last call, in order.
    pub fn take_events(&mut self) -> impl Iterator<Item = GObjEvent> + '_ {
        let len = std::mem::take(&mut self.log_len);
        self.log[..len].iter().flatten().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_object_takes_the_address_freed_last() {
        let mut addresses = GObjAddresses::default();
        let create = |a: &mut GObjAddresses| a.create(GObjClass::Item);
        let (a, b, c) = (
            create(&mut addresses),
            create(&mut addresses),
            create(&mut addresses),
        );
        assert!(a != b && b != c && a != c);
        addresses.destroy(GObjClass::Item, a);
        addresses.destroy(GObjClass::Item, c);
        assert_eq!(addresses.create(GObjClass::Effect), c);
        assert_eq!(addresses.create(GObjClass::Item), a);
        let d = addresses.create(GObjClass::Item);
        assert!(d != a && d != b && d != c);
    }

    #[test]
    fn an_object_from_before_the_match_frees_a_new_address() {
        let mut addresses = GObjAddresses::default();
        let a = addresses.create(GObjClass::Item);
        let banner = addresses.outside();
        assert_ne!(a, banner);
        addresses.destroy(GObjClass::Hud, banner);
        assert_eq!(addresses.create(GObjClass::Item), banner);
    }

    #[test]
    fn the_log_lists_a_tick_in_order_and_empties() {
        let mut addresses = GObjAddresses::default();
        let a = addresses.create(GObjClass::Item);
        addresses.destroy(GObjClass::Item, a);
        let events: Vec<_> = addresses.take_events().collect();
        assert_eq!(
            events,
            [
                GObjEvent {
                    created: true,
                    class: GObjClass::Item,
                    address: a
                },
                GObjEvent {
                    created: false,
                    class: GObjClass::Item,
                    address: a
                },
            ]
        );
        assert_eq!(addresses.take_events().count(), 0);
    }
}

//! Generational slab used for both GObjs and GObjProcs.
//!
//! The C allocates both from `HSD_ObjAlloc` pools (`gobj_alloc_data`,
//! `gobjproc_alloc_data`, gobjinit.c:63-64) whose free list is LIFO
//! (objalloc.c:119-126: `HSD_ObjFree` pushes on `freehead`). Addresses are not
//! part of the observable state we compare, but the free list here is LIFO
//! too so slot reuse follows the same pattern.
//!
//! A [`Key`] is `(index, generation)`. Removing a value bumps the slot's
//! generation, so a key held past the object's destruction never resolves to
//! the slot's next occupant.

/// Index plus generation. Wrapped by the typed ids in `world.rs`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct Key {
    pub index: u32,
    pub generation: u32,
}

struct Slot<T> {
    generation: u32,
    value: Option<T>,
}

pub struct Slab<T> {
    slots: Vec<Slot<T>>,
    /// LIFO free list of slot indices.
    free: Vec<u32>,
    live: usize,
}

impl<T> Default for Slab<T> {
    fn default() -> Self {
        Slab {
            slots: Vec::new(),
            free: Vec::new(),
            live: 0,
        }
    }
}

impl<T> Slab<T> {
    pub fn insert(&mut self, value: T) -> Key {
        self.live += 1;
        if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            debug_assert!(slot.value.is_none());
            slot.value = Some(value);
            return Key {
                index,
                generation: slot.generation,
            };
        }
        let index = u32::try_from(self.slots.len()).expect("slab index overflow");
        self.slots.push(Slot {
            generation: 0,
            value: Some(value),
        });
        Key {
            index,
            generation: 0,
        }
    }

    pub fn remove(&mut self, key: Key) -> Option<T> {
        let slot = self.slots.get_mut(key.index as usize)?;
        if slot.generation != key.generation {
            return None;
        }
        let value = slot.value.take()?;
        slot.generation = slot.generation.wrapping_add(1);
        self.free.push(key.index);
        self.live -= 1;
        Some(value)
    }

    pub fn get(&self, key: Key) -> Option<&T> {
        let slot = self.slots.get(key.index as usize)?;
        if slot.generation != key.generation {
            return None;
        }
        slot.value.as_ref()
    }

    pub fn get_mut(&mut self, key: Key) -> Option<&mut T> {
        let slot = self.slots.get_mut(key.index as usize)?;
        if slot.generation != key.generation {
            return None;
        }
        slot.value.as_mut()
    }

    pub fn contains(&self, key: Key) -> bool {
        self.get(key).is_some()
    }

    /// Number of live values.
    pub fn len(&self) -> usize {
        self.live
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_go_stale_on_remove_and_slots_reuse_lifo() {
        let mut s = Slab::default();
        let a = s.insert("a");
        let b = s.insert("b");
        assert_eq!(s.len(), 2);
        assert_eq!(s.remove(a), Some("a"));
        assert_eq!(s.get(a), None);
        assert_eq!(s.remove(a), None);
        assert_eq!(s.remove(b), Some("b"));
        // LIFO: b's slot (index 1) is handed out first, then a's.
        let c = s.insert("c");
        let d = s.insert("d");
        assert_eq!(c.index, 1);
        assert_eq!(d.index, 0);
        assert_eq!(c.generation, 1);
        assert_eq!(d.generation, 1);
        assert_eq!(s.get(b), None);
        assert_eq!(s.get(c), Some(&"c"));
        assert_eq!(s.len(), 2);
    }
}

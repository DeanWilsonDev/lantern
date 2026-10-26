//! Generational handles and the pool that hands them out.
//!
//! A handle is `(index, generation)`. Removing a value bumps its slot's generation, so an old
//! handle to that slot stops resolving instead of silently pointing at whatever reuses the slot.
//! Iteration is always in slot order, which keeps update order deterministic.
//!
//! Translation note: this is two integers and a comparison in any language.

use std::fmt;
use std::marker::PhantomData;

pub struct Handle<T> {
    index: u32,
    generation: u32,
    marker: PhantomData<fn() -> T>,
}

impl<T> Handle<T> {
    pub fn index(self) -> u32 {
        self.index
    }

    pub fn generation(self) -> u32 {
        self.generation
    }

    /// The same slot address under another type. A backend uses this to hand out public handle
    /// types (`MeshHandle`) for values it keeps in a private pool.
    pub fn cast<U>(self) -> Handle<U> {
        Handle { index: self.index, generation: self.generation, marker: PhantomData }
    }

    /// Packs the handle into one integer, for passing through a script boundary.
    pub fn to_bits(self) -> u64 {
        (u64::from(self.generation) << 32) | u64::from(self.index)
    }

    /// The inverse of `to_bits`. The result still has to be checked against a pool.
    pub fn from_bits(bits: u64) -> Handle<T> {
        Handle { index: bits as u32, generation: (bits >> 32) as u32, marker: PhantomData }
    }
}

// Manual impls so `Handle<T>` is Copy/Eq/Hash whatever `T` is.
impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Handle<T> {}
impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.generation == other.generation
    }
}
impl<T> Eq for Handle<T> {}
impl<T> std::hash::Hash for Handle<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        self.generation.hash(state);
    }
}
impl<T> PartialOrd for Handle<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<T> Ord for Handle<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.index, self.generation).cmp(&(other.index, other.generation))
    }
}
impl<T> fmt::Debug for Handle<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Handle({}v{})", self.index, self.generation)
    }
}

struct Slot<T> {
    generation: u32,
    value: Option<T>,
}

/// Owns values of type `T` and addresses them by `Handle<T>`.
pub struct Pool<T> {
    slots: Vec<Slot<T>>,
    free_indices: Vec<u32>,
    live_count: usize,
}

impl<T> Default for Pool<T> {
    fn default() -> Self {
        Pool { slots: Vec::new(), free_indices: Vec::new(), live_count: 0 }
    }
}

impl<T> Pool<T> {
    pub fn new() -> Pool<T> {
        Pool::default()
    }

    pub fn insert(&mut self, value: T) -> Handle<T> {
        self.live_count += 1;
        if let Some(index) = self.free_indices.pop() {
            let slot = &mut self.slots[index as usize];
            slot.value = Some(value);
            return Handle { index, generation: slot.generation, marker: PhantomData };
        }
        let index = u32::try_from(self.slots.len()).expect("a pool holds fewer than 2^32 values");
        self.slots.push(Slot { generation: 0, value: Some(value) });
        Handle { index, generation: 0, marker: PhantomData }
    }

    pub fn contains(&self, handle: Handle<T>) -> bool {
        self.get(handle).is_some()
    }

    pub fn get(&self, handle: Handle<T>) -> Option<&T> {
        let slot = self.slots.get(handle.index as usize)?;
        if slot.generation != handle.generation {
            return None;
        }
        slot.value.as_ref()
    }

    pub fn get_mut(&mut self, handle: Handle<T>) -> Option<&mut T> {
        let slot = self.slots.get_mut(handle.index as usize)?;
        if slot.generation != handle.generation {
            return None;
        }
        slot.value.as_mut()
    }

    pub fn remove(&mut self, handle: Handle<T>) -> Option<T> {
        let slot = self.slots.get_mut(handle.index as usize)?;
        if slot.generation != handle.generation {
            return None;
        }
        let value = slot.value.take()?;
        slot.generation = slot.generation.wrapping_add(1);
        self.free_indices.push(handle.index);
        self.live_count -= 1;
        Some(value)
    }

    pub fn len(&self) -> usize {
        self.live_count
    }

    pub fn is_empty(&self) -> bool {
        self.live_count == 0
    }

    /// Live values in slot order.
    pub fn iter(&self) -> impl Iterator<Item = (Handle<T>, &T)> {
        self.slots.iter().enumerate().filter_map(|(index, slot)| {
            slot.value
                .as_ref()
                .map(|value| (Handle { index: index as u32, generation: slot.generation, marker: PhantomData }, value))
        })
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Handle<T>, &mut T)> {
        self.slots.iter_mut().enumerate().filter_map(|(index, slot)| {
            let generation = slot.generation;
            slot.value.as_mut().map(|value| (Handle { index: index as u32, generation, marker: PhantomData }, value))
        })
    }

    /// Removes and returns every value, in slot order.
    pub fn drain(&mut self) -> Vec<T> {
        let handles: Vec<Handle<T>> = self.iter().map(|(handle, _)| handle).collect();
        handles.into_iter().filter_map(|handle| self.remove(handle)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_handles_stop_resolving_after_removal() {
        let mut pool = Pool::new();
        let first = pool.insert("first");
        assert_eq!(pool.remove(first), Some("first"));
        let second = pool.insert("second");
        assert_eq!(first.index(), second.index(), "the slot is reused");
        assert_eq!(pool.get(first), None, "but the old handle is stale");
        assert_eq!(pool.get(second), Some(&"second"));
        assert_eq!(pool.remove(first), None);
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn iteration_is_in_slot_order() {
        let mut pool = Pool::new();
        let handles: Vec<_> = (0..5).map(|value| pool.insert(value)).collect();
        pool.remove(handles[1]);
        pool.remove(handles[3]);
        pool.insert(10);
        let values: Vec<i32> = pool.iter().map(|(_, value)| *value).collect();
        // The most recently freed slot (3) is reused first; iteration still walks slots in order.
        assert_eq!(values, vec![0, 2, 10, 4]);
    }

    #[test]
    fn bits_round_trip() {
        let mut pool = Pool::new();
        let handle = pool.insert(1);
        pool.remove(handle);
        let handle = pool.insert(2);
        assert_eq!(Handle::<i32>::from_bits(handle.to_bits()), handle);
    }
}

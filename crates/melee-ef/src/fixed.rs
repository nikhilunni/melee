//! Small inline collections; capacity exhaustion is explicit, never heap growth.
#[derive(Clone, Debug, PartialEq)]
pub struct FixedVec<T, const N: usize> {
    entries: [Option<T>; N],
    len: usize,
}
impl<T, const N: usize> Default for FixedVec<T, N> {
    fn default() -> Self {
        Self {
            entries: std::array::from_fn(|_| None),
            len: 0,
        }
    }
}
impl<T, const N: usize> FixedVec<T, N> {
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn push(&mut self, value: T) {
        assert!(self.len < N, "effect storage capacity {N} exhausted");
        self.entries[self.len] = Some(value);
        self.len += 1;
    }
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        self.entries[self.len].take()
    }
    pub fn remove(&mut self, index: usize) -> T {
        assert!(index < self.len);
        let value = self.entries[index].take().unwrap();
        self.entries[index..self.len].rotate_left(1);
        self.len -= 1;
        value
    }
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &T> {
        self.entries[..self.len].iter().map(|v| v.as_ref().unwrap())
    }
    pub fn iter_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut T> {
        self.entries[..self.len]
            .iter_mut()
            .map(|v| v.as_mut().unwrap())
    }
}

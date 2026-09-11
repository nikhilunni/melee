//! Ownership helpers for preallocated runtime storage.

/// Clone elements while retaining the source's spare capacity. Unlike Vec's
/// Clone, this preserves an initialization-time allocation budget after a fork.
#[allow(clippy::ptr_arg)] // The capacity, not just the elements, is part of the contract.
pub fn clone_vec<T: Clone>(source: &Vec<T>) -> Vec<T> {
    let mut result = Vec::with_capacity(source.capacity());
    result.extend(source.iter().cloned());
    result
}

/// Generalization of Clone which supports dynamically-sized or otherwise unsized types.
///
/// Implemented for:
/// - Normal sized T which implements Clone.
/// - the dynamicaly-sized slice type `[T]`, as long as T itself implements Clone.
/// - the tynamically-sized string slice type `str`.
///
/// This trait could easily be implemented for any other dynamically-sized type as well.
pub trait CloneUnsized {

    /// Mutates `self` to become a clone of `source`.
    ///
    /// Signature purposefully matches `Clone::clone_from`
    /// and `std::slice::clone_from_slice()`.
    fn unsized_clone_from(&mut self, source: &Self);
}

impl<T> CloneUnsized for [T]
where
T: Clone
{
    fn unsized_clone_from(&mut self, source: &Self) {
        self.clone_from_slice(source)
    }
}

impl CloneUnsized for str
{
    fn unsized_clone_from(&mut self, source: &Self) {
        // SAFETY: Cloning valid UTF8 bytes will result in valid UTF8 bytes
        unsafe { self.as_bytes_mut() }.clone_from_slice(source.as_bytes())
    }
}

/// Blanket implementation for any sized T that uses the normal Clone.
impl<T: Clone> CloneUnsized for T {
    fn unsized_clone_from(&mut self, source: &Self) {
        *self = source.clone();
    }
}

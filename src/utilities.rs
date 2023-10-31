/// Like a Cow but not for writing
pub(crate) enum MaybeOwned<'a, T> {
    Borrowed(&'a T),
    Owned(T),
}

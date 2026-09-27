/// Fold a surface Word name to its dictionary key: names resolve
/// case-insensitively (LANG.DICTIONARY.RESOLUTION), and a Word has exactly one
/// name, so case is the only thing folded.
///
/// This is called on every word dispatch, so it allocates only when folding is
/// actually required: an already-uppercase ASCII name (`MAP`, `LENGTH`, most
/// User Words) is its own key and is borrowed unchanged. The borrow is gated on
/// `is_ascii()` so it never diverges from Unicode `to_uppercase` for exotic
/// input.
pub fn canonical_word_name(name: &str) -> std::borrow::Cow<'_, str> {
    if name.is_ascii() && !name.bytes().any(|b| b.is_ascii_lowercase()) {
        return std::borrow::Cow::Borrowed(name);
    }
    std::borrow::Cow::Owned(name.to_uppercase())
}

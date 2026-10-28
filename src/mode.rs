//! Built-in template policies for declared modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OnNoMatch {
    TextOnlyCopy,
    ShallowCopy,
    DeepCopy,
    ShallowSkip,
    DeepSkip,
    Fail,
}
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Mode {
    pub on_no_match: Option<OnNoMatch>,
    pub fail_on_multiple: Option<bool>,
}
impl Mode {
    pub fn policy(self) -> OnNoMatch {
        self.on_no_match.unwrap_or(OnNoMatch::TextOnlyCopy)
    }
}

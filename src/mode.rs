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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Visibility {
    Public,
    Private,
    Final,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Typed {
    Unspecified,
    Untyped,
    Required,
    Lax,
    Strict,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AccumulatorUse {
    All,
    Names(std::collections::BTreeSet<String>),
}
#[derive(Debug, Default, Clone)]
pub(crate) struct Mode {
    pub on_no_match: Option<OnNoMatch>,
    pub fail_on_multiple: Option<bool>,
    pub visibility: Option<Visibility>,
    pub typed: Option<Typed>,
    pub accumulators: Option<AccumulatorUse>,
}
impl Mode {
    pub fn policy(&self) -> OnNoMatch {
        self.on_no_match.unwrap_or(OnNoMatch::TextOnlyCopy)
    }
}

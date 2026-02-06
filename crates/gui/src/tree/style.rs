use sycamore_reactive::{MaybeDyn, ReadSignal, Signal};
use taffy::Style;

#[derive(Clone, PartialEq, Debug, Default)]
pub struct StyleWrapper(pub Style);
// SAFETY: We do not use calc anywhere so it is safe for Style to be send.
unsafe impl Send for StyleWrapper {}
// SAFETY: We do not use calc anywhere so it is safe for Style to be sync.
unsafe impl Sync for StyleWrapper {}

impl From<Style> for StyleWrapper {
    fn from(value: Style) -> Self {
        Self(value)
    }
}
impl From<StyleWrapper> for Style {
    fn from(value: StyleWrapper) -> Self {
        value.0
    }
}

impl From<StyleWrapper> for MaybeDyn<StyleWrapper> {
    fn from(value: StyleWrapper) -> Self {
        MaybeDyn::Static(value)
    }
}

pub struct MaybeDynStyle(MaybeDyn<StyleWrapper>);
impl MaybeDynStyle {
    #[inline(always)]
    pub fn get(self) -> MaybeDyn<StyleWrapper> {
        self.0
    }
}

impl<T: Into<StyleWrapper>> From<T> for MaybeDynStyle {
    fn from(value: T) -> Self {
        MaybeDynStyle(MaybeDyn::Static(value.into()))
    }
}

impl From<Signal<StyleWrapper>> for MaybeDynStyle {
    fn from(value: Signal<StyleWrapper>) -> Self {
        let (read_signal, _) = value.split();
        MaybeDynStyle(MaybeDyn::Signal(read_signal))
    }
}
impl From<ReadSignal<StyleWrapper>> for MaybeDynStyle {
    fn from(value: ReadSignal<StyleWrapper>) -> Self {
        MaybeDynStyle(MaybeDyn::Signal(value))
    }
}

// TODO: some macro for style generation (tailwind-eqsue)?

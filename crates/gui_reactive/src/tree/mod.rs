use graphics::{Mesh, Systems, Vertex};
use input::{KeyboardEvent, MouseEvent};
use sycamore_reactive::{MaybeDyn, ReadSignal, Signal};
use taffy::{AvailableSpace, Layout, Size};

use crate::{
    events::{BlurEvent, EventContext, EventHandler, FocusEvent},
    reexports::reactive::{maybe_get_clone_untracked, maybe_get_untracked},
    zindex::ZIndexProperties,
    ElementId,
};

pub mod builder;

pub trait Node {
    fn render(&mut self, mesh: &mut Mesh<Vertex>, systems: &mut Systems, layout: &Layout);
    fn measure(
        &mut self,
        known_dimensions: Size<Option<f32>>,
        available: Size<AvailableSpace>,
        style: &taffy::Style,
    ) -> Size<f32>;
    fn debug_label(&self) -> &'static str;

    fn children(&self) -> &Vec<ElementId>;

    // General events
    fn mouse_event(&mut self, ctx: &mut EventContext<MouseEvent>);
    fn keyboard_event(&mut self, ctx: &mut EventContext<KeyboardEvent>);
    fn focus_event(&mut self, ctx: &mut EventContext<FocusEvent>);
    fn blur_event(&mut self, ctx: &mut EventContext<BlurEvent>);

    // Rendering
    fn get_zindex_properties(&self) -> ZIndexProperties;
    // Taffy specific
    fn get_style(&self) -> taffy::Style;

    fn get_unrounded_layout(&self) -> &taffy::Layout;
    fn get_final_layout(&self) -> &taffy::Layout;

    fn set_unrounded_layout(&mut self, layout: Layout);
    fn set_final_layout(&mut self, layout: Layout);

    fn layout_cache(&self) -> &taffy::Cache;
    fn layout_cache_mut(&mut self) -> &mut taffy::Cache;

    fn node_id(&self) -> ElementId;
}

pub trait Widget {
    fn render(
        &mut self,
        mesh: &mut Mesh<Vertex>,
        systems: &mut Systems,
        layout: &Layout,
        style: &taffy::Style,
    );

    fn measure(
        &mut self,
        known_dimensions: Size<Option<f32>>,
        available: Size<AvailableSpace>,
        style: &taffy::Style,
    ) -> Size<f32>;

    fn debug_label(&self) -> &'static str;

    fn focusable(&self) -> bool;
}

pub struct Element {
    node_id: ElementId,

    pub inner: Box<dyn Widget>,
    style: MaybeDyn<StyleWrapper>,
    zindex: MaybeDyn<ZIndexProperties>,

    // taffy relative layouts
    rel_unrounded_layout: taffy::Layout,
    rel_final_layout: taffy::Layout,
    // absolute layout
    layout: taffy::Layout,

    cache: taffy::Cache,

    children: Vec<ElementId>,

    pub(crate) mouse_handler: EventHandler<MouseEvent>,
    pub(crate) keyboard_handler: EventHandler<KeyboardEvent>,
    pub(crate) focus_handler: EventHandler<FocusEvent>,
    pub(crate) blur_handler: EventHandler<BlurEvent>,
}
impl std::fmt::Debug for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Element")
            .field("node_id", &self.node_id)
            .field("inner", &self.inner.debug_label())
            .field("rel_unrounded_layout", &self.rel_unrounded_layout)
            .field("rel_final_layout", &self.rel_final_layout)
            .field("layout", &self.layout)
            .field("cache", &self.cache)
            .field("children", &self.children)
            .finish()
    }
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct StyleWrapper(taffy::Style);
// SAFETY: We do not use calc anywhere so it is safe for Style to be send.
unsafe impl Send for StyleWrapper {}
unsafe impl Sync for StyleWrapper {}
impl From<taffy::Style> for StyleWrapper {
    fn from(value: taffy::Style) -> Self {
        Self(value)
    }
}
impl From<StyleWrapper> for taffy::Style {
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

impl Node for Element {
    #[inline(always)]
    fn render(&mut self, mesh: &mut Mesh<Vertex>, systems: &mut Systems, layout: &Layout) {
        self.inner.render(
            mesh,
            systems,
            layout,
            &maybe_get_clone_untracked(&self.style).into(),
        );
    }
    #[inline(always)]
    fn measure(
        &mut self,
        known_dimensions: Size<Option<f32>>,
        available_space: Size<AvailableSpace>,
        style: &taffy::Style,
    ) -> Size<f32> {
        self.inner.measure(known_dimensions, available_space, style)
    }
    #[inline(always)]
    fn debug_label(&self) -> &'static str {
        self.inner.debug_label()
    }

    fn children(&self) -> &Vec<ElementId> {
        &self.children
    }

    #[inline(always)]
    fn mouse_event(&mut self, ctx: &mut EventContext<MouseEvent>) {
        self.mouse_handler.handle(self, ctx);
    }
    #[inline(always)]
    fn keyboard_event(&mut self, ctx: &mut EventContext<KeyboardEvent>) {
        self.keyboard_handler.handle(self, ctx);
    }

    #[inline(always)]
    fn focus_event(&mut self, ctx: &mut EventContext<FocusEvent>) {
        self.focus_handler.handle(self, ctx);
    }
    #[inline(always)]
    fn blur_event(&mut self, ctx: &mut EventContext<BlurEvent>) {
        self.blur_handler.handle(self, ctx);
    }

    fn get_zindex_properties(&self) -> ZIndexProperties {
        maybe_get_untracked(&self.zindex)
    }

    #[inline(always)]
    fn get_style(&self) -> taffy::Style {
        maybe_get_clone_untracked(&self.style).into()
    }

    #[inline(always)]
    fn get_unrounded_layout(&self) -> &taffy::Layout {
        &self.rel_unrounded_layout
    }
    #[inline(always)]
    fn get_final_layout(&self) -> &taffy::Layout {
        &self.rel_final_layout
    }

    #[inline(always)]
    fn set_unrounded_layout(&mut self, layout: Layout) {
        self.rel_unrounded_layout = layout;
    }
    #[inline(always)]
    fn set_final_layout(&mut self, layout: taffy::Layout) {
        self.rel_final_layout = layout;
    }

    #[inline(always)]
    fn layout_cache(&self) -> &taffy::Cache {
        &self.cache
    }
    #[inline(always)]
    fn layout_cache_mut(&mut self) -> &mut taffy::Cache {
        &mut self.cache
    }

    fn node_id(&self) -> ElementId {
        self.node_id
    }
}

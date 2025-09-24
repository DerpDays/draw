use std::{
    ptr::NonNull,
    sync::{Mutex, OnceLock},
    time::Duration,
};

use graphics::{Mesh, Systems, Vertex};
use input::{KeyboardEvent, MouseEvent};
use reactive_graph::{signal::ArcReadSignal, traits::GetUntracked};
use taffy::{AvailableSpace, Layout, Size};

mod for_each;
pub use for_each::{NodeForEach, NodeVisitor};

mod into_view;
pub use into_view::IntoView;

mod add_child;
pub use add_child::ElementChild;

use crate::{
    events::{BlurEvent, EventContext, EventHandler, FocusEvent},
    zindex::ZIndexProperties,
};

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub struct DynNodeId(NonNull<dyn DynNode + Send + Sync>);
unsafe impl Send for DynNodeId {}
unsafe impl Sync for DynNodeId {}

impl std::fmt::Debug for DynNodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(format!("DynNodeId: {}", unsafe { self.0.as_ref() }.debug_label()).as_str())
            .finish()
    }
}
impl DynNodeId {
    pub const unsafe fn as_ref(&self) -> &dyn DynNode {
        unsafe { self.0.as_ref() }
    }
    pub const unsafe fn as_mut(&mut self) -> &mut dyn DynNode {
        unsafe { self.0.as_mut() }
    }
}

pub trait Node: DynNode {
    type Children: NodeForEach;

    fn children(&self) -> &Self::Children;
    fn children_mut(&mut self) -> &mut Self::Children;
}

pub trait DynNode: Send + Sync {
    fn render(&mut self, mesh: &mut Mesh<Vertex>, systems: &mut Systems);
    fn measure(
        &mut self,
        known_dimensions: Size<Option<f32>>,
        available: Size<AvailableSpace>,
        style: &taffy::Style,
    ) -> Size<f32>;
    fn debug_label(&self) -> &'static str;

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

    fn node_id(&self) -> DynNodeId
    where
        Self: Sized + 'static,
    {
        DynNodeId(NonNull::from(self as &dyn DynNode))
    }
}

pub trait Widget: Send + Sync {
    fn render(&mut self, mesh: &mut Mesh<Vertex>, systems: &mut Systems, layout: &Layout);

    fn measure(
        &mut self,
        known_dimensions: Size<Option<f32>>,
        available: Size<AvailableSpace>,
        style: &taffy::Style,
    ) -> Size<f32>;

    fn debug_label(&self) -> &'static str;

    fn focusable() -> bool;
}

/// Bounds trait used to mark which elements can have children.
pub(crate) trait ElementWithChildren {}

pub struct Element<T, C: NodeForEach> {
    style: StyleSource,
    final_layout: taffy::Layout,
    unrounded_layout: taffy::Layout,
    cache: taffy::Cache,

    zindex: ZIndexProperties,

    pub inner: T,
    pub children: C,

    focusable: bool,

    pub(crate) mouse_handler: EventHandler<MouseEvent>,
    pub(crate) keyboard_handler: EventHandler<KeyboardEvent>,
    pub(crate) focus_handler: EventHandler<FocusEvent>,
    pub(crate) blur_handler: EventHandler<BlurEvent>,
}

pub enum StyleSource {
    NonReactive(taffy::Style),
    Reactive(ArcReadSignal<taffy::Style>),
}
impl StyleSource {
    #[inline]
    pub fn get(&self) -> taffy::Style {
        match self {
            StyleSource::NonReactive(style) => style.clone(),
            StyleSource::Reactive(signal) => signal.get_untracked(),
        }
    }
}

impl Default for StyleSource {
    fn default() -> Self {
        Self::NonReactive(taffy::Style::DEFAULT)
    }
}
impl From<ArcReadSignal<taffy::Style>> for StyleSource {
    fn from(value: ArcReadSignal<taffy::Style>) -> Self {
        Self::Reactive(value)
    }
}
impl From<taffy::Style> for StyleSource {
    fn from(value: taffy::Style) -> Self {
        Self::NonReactive(value)
    }
}
// SAFETY: We do not use calc anywhere so it is safe for Style to be send.
unsafe impl Send for StyleSource {}
unsafe impl Sync for StyleSource {}

// #[derive(Default)]
// pub struct Style(pub taffy::Style);
// // SAFETY: We do not use calc anywhere so it is safe for Style to be send.
// unsafe impl Send for Style {}

impl<T: Widget, C: NodeForEach> Node for Element<T, C> {
    type Children = C;

    fn children(&self) -> &Self::Children {
        &self.children
    }
    fn children_mut(&mut self) -> &mut Self::Children {
        &mut self.children
    }
}

impl<T: Widget, C: NodeForEach> DynNode for Element<T, C> {
    #[inline(always)]
    fn render(&mut self, mesh: &mut Mesh<Vertex>, systems: &mut Systems) {
        self.inner.render(mesh, systems, &self.final_layout);
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

    #[inline(always)]
    fn mouse_event(&mut self, ctx: &mut EventContext<MouseEvent>) {
        self.mouse_handler.handle(ctx);
    }
    #[inline(always)]
    fn keyboard_event(&mut self, ctx: &mut EventContext<KeyboardEvent>) {
        self.keyboard_handler.handle(ctx);
    }

    #[inline(always)]
    fn focus_event(&mut self, ctx: &mut EventContext<FocusEvent>) {
        self.focus_handler.handle(ctx);
    }
    #[inline(always)]
    fn blur_event(&mut self, ctx: &mut EventContext<BlurEvent>) {
        self.blur_handler.handle(ctx);
    }

    fn get_zindex_properties(&self) -> ZIndexProperties {
        self.zindex
    }

    #[inline(always)]
    fn get_style(&self) -> taffy::Style {
        self.style.get()
    }

    #[inline(always)]
    fn get_unrounded_layout(&self) -> &taffy::Layout {
        &self.unrounded_layout
    }
    #[inline(always)]
    fn get_final_layout(&self) -> &taffy::Layout {
        &self.final_layout
    }

    #[inline(always)]
    fn set_unrounded_layout(&mut self, layout: Layout) {
        self.unrounded_layout = layout;
    }
    #[inline(always)]
    fn set_final_layout(&mut self, layout: taffy::Layout) {
        self.final_layout = layout;
    }

    #[inline(always)]
    fn layout_cache(&self) -> &taffy::Cache {
        &self.cache
    }
    #[inline(always)]
    fn layout_cache_mut(&mut self) -> &mut taffy::Cache {
        &mut self.cache
    }
}

impl<T: Widget> Element<T, ()> {
    pub fn new_empty(inner: T) -> Self {
        Self {
            style: StyleSource::default(),
            final_layout: Layout::new(),
            unrounded_layout: Layout::new(),
            cache: taffy::Cache::new(),

            zindex: ZIndexProperties::DEFAULT,

            inner,
            children: (),

            focusable: T::focusable(),

            mouse_handler: EventHandler::empty(),
            keyboard_handler: EventHandler::empty(),
            focus_handler: EventHandler::empty(),
            blur_handler: EventHandler::empty(),
        }
    }
}

impl<T: Widget, C: NodeForEach> Element<T, C> {
    /// Assign a style to this element.
    ///
    /// ```rust
    /// div().style(Style::DEFAULT);
    /// ```
    pub fn style<S: Into<StyleSource>>(mut self, style: S) -> Self {
        self.style = style.into();
        self
    }

    /// Assign a z index properties to this element.
    ///
    /// ```rust
    /// div().zindex(ZIndexProperties::DEFAULT);
    /// ```
    pub fn zindex(mut self, zindex: ZIndexProperties) -> Self {
        self.zindex = zindex;
        self
    }
}

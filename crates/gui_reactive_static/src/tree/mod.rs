use std::ptr::NonNull;

use graphics::{Mesh, Systems, Vertex};
use input::{KeyboardEvent, MouseEvent};
use reactive_graph::{
    computed::{ArcMemo, Memo},
    effect::Effect,
    signal::{ArcReadSignal, ReadSignal},
    traits::{Get, GetUntracked},
    wrappers::read::Signal,
};
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
    TreeManager,
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
    fn render(&mut self, mesh: &mut Mesh<Vertex>, systems: &mut Systems, layout: &Layout);
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

    unsafe fn node_id(&self) -> DynNodeId
    where
        Self: Sized + 'static,
    {
        DynNodeId(NonNull::from(self as &dyn DynNode))
    }
}

pub trait Widget: Send + Sync {
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

    fn focusable() -> bool;
}

/// Bounds trait used to mark which elements can have children.
pub(crate) trait ElementWithChildren {}

pub struct Element<T, C: NodeForEach> {
    style: Signal<StyleWrapper>,
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

#[derive(Default, Clone, PartialEq)]
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

pub struct ReactiveStyle(pub Signal<StyleWrapper>);

impl From<Signal<StyleWrapper>> for ReactiveStyle {
    fn from(value: Signal<StyleWrapper>) -> Self {
        Self(value)
    }
}

impl From<ArcReadSignal<StyleWrapper>> for ReactiveStyle {
    fn from(value: ArcReadSignal<StyleWrapper>) -> Self {
        Self(value.into())
    }
}
impl From<ReadSignal<StyleWrapper>> for ReactiveStyle {
    fn from(value: ReadSignal<StyleWrapper>) -> Self {
        Self(value.into())
    }
}
impl From<ArcMemo<StyleWrapper>> for ReactiveStyle {
    fn from(value: ArcMemo<StyleWrapper>) -> Self {
        Self(value.into())
    }
}
impl From<Memo<StyleWrapper>> for ReactiveStyle {
    fn from(value: Memo<StyleWrapper>) -> Self {
        Self(value.into())
    }
}

impl From<StyleWrapper> for ReactiveStyle {
    fn from(value: StyleWrapper) -> Self {
        Self(value.into())
    }
}

impl From<taffy::Style> for ReactiveStyle {
    fn from(value: taffy::Style) -> Self {
        Self(StyleWrapper(value).into())
    }
}

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
    fn render(&mut self, mesh: &mut Mesh<Vertex>, systems: &mut Systems, layout: &Layout) {
        self.inner
            .render(mesh, systems, layout, &self.style.get_untracked().into());
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
        self.zindex
    }

    #[inline(always)]
    fn get_style(&self) -> taffy::Style {
        self.style.get_untracked().into()
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
            style: StyleWrapper::default().into(),
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
    pub(crate) fn update_inner(self, inner: T) -> Self {
        Self {
            style: self.style,
            final_layout: self.final_layout,
            unrounded_layout: self.unrounded_layout,
            cache: self.cache,

            zindex: self.zindex,

            inner: inner,
            children: self.children,

            focusable: self.focusable,

            mouse_handler: self.mouse_handler,
            keyboard_handler: self.keyboard_handler,
            focus_handler: self.focus_handler,
            blur_handler: self.blur_handler,
        }
    }
}

impl<T: Widget + 'static, C: NodeForEach + 'static> Element<T, C> {
    /// Assign a style to this element.
    ///
    /// ```rust
    /// div().style(Style::DEFAULT);
    /// ```
    pub fn style(mut self, style: impl Into<ReactiveStyle>) -> Self {
        let style = style.into().0;
        self.style = style;

        tracing::info!("inside style fn");
        let mgr = TreeManager::global();
        Effect::watch_sync(
            move || style.get(),
            move |new, old, _| {
                if Some(new) != old {
                    tracing::warn!("calling relayout from style!!");
                    mgr.relayout(node_id);
                    mgr.now();
                }
            },
            false,
        );
        tracing::info!("added effect");

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

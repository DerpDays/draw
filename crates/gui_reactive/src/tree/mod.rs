use std::{any::Any, sync::Arc};

use graphics::{Mesh, Systems, Vertex};
use input::{KeyboardEvent, MouseEvent};
use reactive_graph::{
    computed::{ArcMemo, Memo},
    effect::Effect,
    signal::{ArcReadSignal, ReadSignal},
    traits::{Get, GetUntracked},
    wrappers::read::Signal,
};
use slotmap::SlotMap;
use taffy::{AvailableSpace, Layout, Size};

// mod for_each;
// pub use for_each::{NodeForEach, NodeVisitor};

use crate::{
    events::{BlurEvent, EventContext, EventHandler, FocusEvent},
    zindex::ZIndexProperties,
    ElementId, TreeManager,
};

pub trait Node: Send + Sync {
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

    fn focusable(&self) -> bool;

    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub struct Element {
    node_id: ElementId,

    pub inner: Box<dyn Widget>,
    style: Signal<StyleWrapper>,
    zindex: Signal<ZIndexProperties>,

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
impl Element {
    pub fn inner_as_any(&self) -> &dyn Any {
        self.inner.as_any()
    }
    pub fn inner_as_any_mut(&mut self) -> &mut dyn Any {
        self.inner.as_any_mut()
    }
}
impl std::fmt::Debug for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Element")
            .field("node_id", &self.node_id)
            .field("inner", &self.inner.debug_label())
            .field("style", &self.style)
            .field("zindex", &self.zindex)
            .field("rel_unrounded_layout", &self.rel_unrounded_layout)
            .field("rel_final_layout", &self.rel_final_layout)
            .field("layout", &self.layout)
            .field("cache", &self.cache)
            .field("children", &self.children)
            .finish()
    }
}

pub trait ErasedBuilder {
    fn build(self: Box<Self>, arena: &mut SlotMap<ElementId, Element>) -> ElementId;
}

pub struct ElementBuilder<W: Widget> {
    inner: W,
    style: Signal<StyleWrapper>,
    zindex: Signal<ZIndexProperties>,

    children: Vec<Box<dyn ErasedBuilder>>,

    pub(crate) mouse_handler: EventHandler<MouseEvent>,
    pub(crate) keyboard_handler: EventHandler<KeyboardEvent>,
    pub(crate) focus_handler: EventHandler<FocusEvent>,
    pub(crate) blur_handler: EventHandler<BlurEvent>,

    pub(crate) after_build: Option<Arc<dyn Fn(ElementId)>>,
}

impl<W: Widget> ElementBuilder<W> {
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            style: StyleWrapper::default().into(),
            zindex: ZIndexProperties::DEFAULT.into(),

            children: Vec::new(),

            mouse_handler: EventHandler::empty(),
            keyboard_handler: EventHandler::empty(),
            focus_handler: EventHandler::empty(),
            blur_handler: EventHandler::empty(),

            after_build: None,
        }
    }

    pub fn new_with_callback<F>(inner: W, after_build: F) -> Self
    where
        F: Fn(ElementId) + Send + Sync + 'static,
    {
        let mut builder = Self::new(inner);
        builder.after_build = Some(Arc::new(after_build) as Arc<dyn Fn(ElementId)>);
        builder
    }

    pub fn append_after_build<F>(mut self, after_build: F) -> Self
    where
        F: Fn(ElementId) + Send + Sync + 'static,
    {
        self.after_build = if let Some(prev_build) = self.after_build {
            Some(Arc::new(move |elem_id| {
                (prev_build)(elem_id);
                (after_build)(elem_id);
            }) as Arc<dyn Fn(ElementId)>)
        } else {
            Some(Arc::new(after_build) as Arc<dyn Fn(ElementId)>)
        };
        self
    }
    pub fn set_inner(mut self, inner: W) -> Self {
        self.inner = inner;
        self
    }

    pub fn inner_mut(&mut self) -> &mut W {
        &mut self.inner
    }
}

impl<W: Widget> ElementBuilder<W> {
    pub fn child(mut self, child: impl Into<BuilderList>) -> Self {
        self.children.extend(child.into().0);
        self
    }

    /// Assign a style to this element.
    ///
    /// ```rust
    /// div().style(Style::DEFAULT);
    /// ```
    pub fn style(mut self, style: impl Into<ReactiveStyle>) -> Self {
        let style = style.into().0;
        self.style = style;
        self.append_after_build(move |elem_id| {
            let mgr = TreeManager::global();
            Effect::watch_sync(
                move || style.get(),
                move |new, old, _| {
                    if Some(new) != old {
                        tracing::warn!("calling relayout from style!!");
                        mgr.relayout(elem_id);
                        mgr.now();
                    }
                },
                false,
            );
        })
    }

    //
    //     /// Assign a z index properties to this element.
    //     ///
    //     /// ```rust
    //     /// div().zindex(ZIndexProperties::DEFAULT);
    //     /// ```
    //     pub fn zindex(mut self, zindex: ZIndexProperties) -> Self {
    //         self.zindex = zindex;
    //         self
    //     }
}
impl<W: Widget + 'static> ElementBuilder<W> {
    pub(crate) fn build(self, arena: &mut SlotMap<ElementId, Element>) -> ElementId {
        let key = arena.insert_with_key(|id| Element {
            node_id: id,

            inner: Box::new(self.inner),
            style: self.style,
            zindex: self.zindex,

            rel_final_layout: Layout::new(),
            rel_unrounded_layout: Layout::new(),
            layout: Layout::new(),
            cache: taffy::Cache::new(),

            children: Vec::new(),

            mouse_handler: self.mouse_handler,
            keyboard_handler: self.keyboard_handler,
            focus_handler: self.focus_handler,
            blur_handler: self.blur_handler,
        });

        // Build children
        for child in self.children {
            let child_id = child.build(arena);
            arena[key].children.push(child_id);
        }

        // Post-build hooks
        if let Some(hook) = self.after_build {
            hook(key);
        }

        key
    }
}

impl<W: Widget + 'static> ErasedBuilder for ElementBuilder<W> {
    fn build(self: Box<Self>, arena: &mut SlotMap<ElementId, Element>) -> ElementId {
        (*self).build(arena)
    }
}

pub struct BuilderList(Vec<Box<dyn ErasedBuilder>>);

impl<T: ErasedBuilder + 'static> From<T> for BuilderList {
    fn from(val: T) -> Self {
        BuilderList(vec![Box::new(val)])
    }
}

impl<T: ErasedBuilder + 'static> From<Vec<T>> for BuilderList {
    fn from(val: Vec<T>) -> Self {
        BuilderList(
            val.into_iter()
                .map(|x| Box::new(x) as Box<dyn ErasedBuilder>)
                .collect::<Vec<_>>(),
        )
    }
}

macro_rules! impl_for_each {
    ( $( ( $( $name:ident ),+ ) ),* ) => {
        $(
            #[allow(unused_parens)]
            impl<$( $name: Into<BuilderList> ),+> From<( $( $name ),+ , )> for BuilderList{
                #[allow(non_snake_case)]
                fn from(( $( $name ),+ , ): ( $( $name ),+ , )) -> BuilderList {
                    let mut list = vec![];
                    $(
                        list.extend($name.into().0);
                    )+
                    BuilderList(list)
                }
            }
        )*
    };
}

// Generate impls for tuple sizes 1 through 16 (or higher if you like)
impl_for_each![
    (A),
    (A, B),
    (A, B, C),
    (A, B, C, D),
    (A, B, C, D, E),
    (A, B, C, D, E, F),
    (A, B, C, D, E, F, G),
    (A, B, C, D, E, F, G, H),
    (A, B, C, D, E, F, G, H, I),
    (A, B, C, D, E, F, G, H, I, J),
    (A, B, C, D, E, F, G, H, I, J, K),
    (A, B, C, D, E, F, G, H, I, J, K, L),
    (A, B, C, D, E, F, G, H, I, J, K, L, M),
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N),
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O),
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P),
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q),
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R),
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S),
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T),
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U),
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V),
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W),
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X),
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y),
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z)
];

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

impl Node for Element {
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
        self.zindex.get_untracked()
    }

    #[inline(always)]
    fn get_style(&self) -> taffy::Style {
        self.style.get_untracked().into()
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

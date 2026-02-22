use euclid::default::Box2D;
use graphics::Primitive;
use input::{KeyboardEvent, MouseEvent};
use taffy::{AvailableSpace, Layout, Size};

use crate::{
    ElementId,
    MeasureCtx,
    events::{BlurEvent, EventContext, EventHandler, FocusEvent},
    prelude::{MaybeDyn, Style},
    zindex::ZIndexProperties,
};

pub trait Node {
    fn render(&mut self, layout: &Layout) -> Option<Primitive>;
    fn measure(
        &mut self,
        measure_ctx: &mut dyn MeasureCtx,
        known_dimensions: Size<Option<f32>>,
        available: Size<AvailableSpace>,
        style: &Style,
    ) -> Size<f32>;
    fn debug_label(&self) -> &'static str;

    fn children(&self) -> &[ElementId];

    // General events
    fn mouse_event(&mut self, ctx: &mut EventContext<MouseEvent>);
    fn keyboard_event(&mut self, ctx: &mut EventContext<KeyboardEvent>);
    fn focus_event(&mut self, ctx: &mut EventContext<FocusEvent>);
    fn blur_event(&mut self, ctx: &mut EventContext<BlurEvent>);

    // Rendering
    fn get_style(&self) -> &MaybeDyn<Style>;
    fn get_style_clone(&self) -> Style;

    fn get_relative_unrounded_layout(&self) -> &taffy::Layout;
    fn get_relative_final_layout(&self) -> &taffy::Layout;
    fn get_abs_layout(&self) -> &taffy::Layout;
    fn get_clip_rect(&self) -> &Box2D<f32>;

    fn set_relative_unrounded_layout(&mut self, layout: Layout);
    fn set_relative_final_layout(&mut self, layout: Layout);
    fn set_abs_layout(&mut self, layout: Layout);
    fn set_clip_rect(&mut self, clip: Box2D<f32>);

    fn layout_cache(&self) -> &taffy::Cache;
    fn layout_cache_mut(&mut self) -> &mut taffy::Cache;

    fn node_id(&self) -> ElementId;
    fn parent_id(&self) -> Option<ElementId>;
}

pub trait Widget {
    fn render(&mut self, layout: &Layout, style: &Style) -> Option<Primitive>;

    /// Measures the size of this element.
    ///
    /// NOTE: this is only ran when the element is a leaf node (i.e. no children).
    fn measure(
        &mut self,
        id: ElementId,
        measure_ctx: &mut dyn MeasureCtx,
        known_dimensions: Size<Option<f32>>,
        available: Size<AvailableSpace>,
        style: &Style,
    ) -> Size<f32>;

    fn debug_label(&self) -> &'static str;

    fn focusable(&self) -> bool;

    #[allow(unused_variables)]
    fn default_mouse_event(&mut self, ctx: &mut EventContext<MouseEvent>, abs_layout: &Layout) {}
    #[allow(unused_variables)]
    fn default_keyboard_event(
        &mut self,
        ctx: &mut EventContext<KeyboardEvent>,
        abs_layout: &Layout,
    ) {
    }
    #[allow(unused_variables)]
    fn default_focus_event(&mut self, ctx: &mut EventContext<FocusEvent>, abs_layout: &Layout) {}
    #[allow(unused_variables)]
    fn default_blur_event(&mut self, ctx: &mut EventContext<BlurEvent>, abs_layout: &Layout) {}
}

pub struct Element {
    pub(crate) node_id: ElementId,
    pub(crate) parent_id: Option<ElementId>,

    pub(crate) inner: Box<dyn Widget>,
    pub(crate) style: MaybeDyn<Style>,

    // taffy relative layouts
    pub(crate) rel_unrounded_layout: taffy::Layout,
    pub(crate) rel_final_layout: taffy::Layout,
    // absolute layout
    pub(crate) abs_layout: taffy::Layout,
    pub(crate) clip_rect: Box2D<f32>,

    pub(crate) cache: taffy::Cache,

    pub(crate) children: Vec<ElementId>,

    pub(crate) mouse_handler: EventHandler<MouseEvent>,
    pub(crate) keyboard_handler: EventHandler<KeyboardEvent>,
    pub(crate) focus_handler: EventHandler<FocusEvent>,
    pub(crate) blur_handler: EventHandler<BlurEvent>,
}
impl std::fmt::Debug for Box<dyn Widget> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Widget")
            .field("label", &self.debug_label())
            .finish()
    }
}
impl std::fmt::Debug for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Element")
            .field("node_id", &self.node_id)
            .field("parent_id", &self.parent_id)
            .field("inner", &self.inner)
            .field("style", &self.get_style_clone())
            .field("rel_unrounded_layout", &self.rel_unrounded_layout)
            .field("rel_final_layout", &self.rel_final_layout)
            .field("abs_layout", &self.abs_layout)
            .field("clip_rect", &self.clip_rect)
            .field("cache", &self.cache)
            .field("children", &self.children)
            .finish()
    }
}

impl Node for Element {
    #[inline]
    fn render(&mut self, layout: &Layout) -> Option<Primitive> {
        self.style
            .with_untracked(|style| self.inner.render(layout, style))
    }
    #[inline]
    fn measure(
        &mut self,
        measure_ctx: &mut dyn MeasureCtx,
        known_dimensions: Size<Option<f32>>,
        available_space: Size<AvailableSpace>,
        style: &Style,
    ) -> Size<f32> {
        self.inner.measure(
            self.node_id,
            measure_ctx,
            known_dimensions,
            available_space,
            style,
        )
    }
    #[inline]
    fn debug_label(&self) -> &'static str {
        self.inner.debug_label()
    }

    fn children(&self) -> &[ElementId] {
        &self.children
    }

    #[inline]
    fn mouse_event(&mut self, ctx: &mut EventContext<MouseEvent>) {
        self.mouse_handler.handle(self, ctx);
        if !ctx.is_preventing_default() {
            self.inner.default_mouse_event(ctx, &self.abs_layout);
        }
    }
    #[inline]
    fn keyboard_event(&mut self, ctx: &mut EventContext<KeyboardEvent>) {
        self.keyboard_handler.handle(self, ctx);
        if !ctx.is_preventing_default() {
            self.inner.default_keyboard_event(ctx, &self.abs_layout);
        }
    }

    #[inline]
    fn focus_event(&mut self, ctx: &mut EventContext<FocusEvent>) {
        self.focus_handler.handle(self, ctx);
        if !ctx.is_preventing_default() {
            self.inner.default_focus_event(ctx, &self.abs_layout);
        }
    }
    #[inline]
    fn blur_event(&mut self, ctx: &mut EventContext<BlurEvent>) {
        self.blur_handler.handle(self, ctx);
        if !ctx.is_preventing_default() {
            self.inner.default_blur_event(ctx, &self.abs_layout);
        }
    }

    #[inline]
    fn get_style_clone(&self) -> Style {
        self.style.get_clone_untracked()
    }
    #[inline]
    fn get_style(&self) -> &MaybeDyn<Style> {
        &self.style
    }

    #[inline]
    fn get_relative_unrounded_layout(&self) -> &taffy::Layout {
        &self.rel_unrounded_layout
    }
    #[inline]
    fn get_relative_final_layout(&self) -> &taffy::Layout {
        &self.rel_final_layout
    }
    #[inline]
    fn get_abs_layout(&self) -> &taffy::Layout {
        &self.abs_layout
    }
    #[inline]
    fn get_clip_rect(&self) -> &Box2D<f32> {
        &self.clip_rect
    }

    #[inline]
    fn set_relative_unrounded_layout(&mut self, layout: Layout) {
        self.rel_unrounded_layout = layout;
    }
    #[inline]
    fn set_relative_final_layout(&mut self, layout: taffy::Layout) {
        self.rel_final_layout = layout;
    }
    #[inline]
    fn set_abs_layout(&mut self, layout: taffy::Layout) {
        self.abs_layout = layout;
    }
    #[inline]
    fn set_clip_rect(&mut self, clip: Box2D<f32>) {
        self.clip_rect = clip;
    }

    #[inline]
    fn layout_cache(&self) -> &taffy::Cache {
        &self.cache
    }
    #[inline]
    fn layout_cache_mut(&mut self) -> &mut taffy::Cache {
        &mut self.cache
    }

    fn node_id(&self) -> ElementId {
        self.node_id
    }
    fn parent_id(&self) -> Option<ElementId> {
        self.parent_id
    }
}

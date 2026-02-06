use graphics::Primitive;
use input::{KeyboardEvent, MouseEvent};
use sycamore_reactive::MaybeDyn;
use taffy::{AvailableSpace, Layout, Size};

use crate::{
    ElementId,
    MeasureCtx,
    events::{BlurEvent, EventContext, EventHandler, FocusEvent},
    reexports::reactivity::{maybe_get_clone_untracked, maybe_get_untracked},
    zindex::ZIndexProperties,
};

pub mod builder;
mod style;
pub use style::{MaybeDynStyle, StyleWrapper};

pub trait Node {
    fn render(&mut self, layout: &Layout) -> Option<Primitive>;
    fn measure(
        &mut self,
        measure_ctx: &mut dyn MeasureCtx,
        known_dimensions: Size<Option<f32>>,
        available: Size<AvailableSpace>,
        style: &taffy::Style,
    ) -> Size<f32>;
    fn debug_label(&self) -> &'static str;

    fn children(&self) -> &[ElementId];

    // General events
    fn mouse_event(&mut self, ctx: &mut EventContext<MouseEvent>);
    fn keyboard_event(&mut self, ctx: &mut EventContext<KeyboardEvent>);
    fn focus_event(&mut self, ctx: &mut EventContext<FocusEvent>);
    fn blur_event(&mut self, ctx: &mut EventContext<BlurEvent>);

    // Rendering
    fn get_zindex_properties(&self) -> ZIndexProperties;
    // Taffy specific
    fn get_style(&self) -> taffy::Style;

    fn get_abs_layout(&self) -> &taffy::Layout;

    fn get_relative_unrounded_layout(&self) -> &taffy::Layout;
    fn get_relative_final_layout(&self) -> &taffy::Layout;

    fn set_relative_unrounded_layout(&mut self, layout: Layout);
    fn set_relative_final_layout(&mut self, layout: Layout);
    fn set_abs_layout(&mut self, layout: Layout);

    fn layout_cache(&self) -> &taffy::Cache;
    fn layout_cache_mut(&mut self) -> &mut taffy::Cache;

    fn node_id(&self) -> ElementId;
    fn parent_id(&self) -> Option<ElementId>;
}

pub trait Widget {
    fn render(&mut self, layout: &Layout, style: &taffy::Style) -> Option<Primitive>;

    /// Measures the size of this element.
    ///
    /// NOTE: this is only ran when the element is a leaf node (i.e. no children).
    fn measure(
        &mut self,
        measure_ctx: &mut dyn MeasureCtx,
        known_dimensions: Size<Option<f32>>,
        available: Size<AvailableSpace>,
        style: &taffy::Style,
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
    node_id: ElementId,
    parent_id: Option<ElementId>,

    pub inner: Box<dyn Widget>,
    style: MaybeDyn<StyleWrapper>,
    // scroll amount in pixels
    scroll_amount: f32,
    zindex: MaybeDyn<ZIndexProperties>,

    // taffy relative layouts
    rel_unrounded_layout: taffy::Layout,
    rel_final_layout: taffy::Layout,
    // absolute layout
    abs_layout: taffy::Layout,

    cache: taffy::Cache,

    pub(crate) children: Vec<ElementId>,

    pub(crate) mouse_handler: EventHandler<MouseEvent>,
    pub(crate) keyboard_handler: EventHandler<KeyboardEvent>,
    pub(crate) focus_handler: EventHandler<FocusEvent>,
    pub(crate) blur_handler: EventHandler<BlurEvent>,
}
impl std::fmt::Debug for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Element")
            .field("node_id", &self.node_id)
            .field("parent_id", &self.parent_id)
            .field("inner", &self.inner.debug_label())
            .field("rel_unrounded_layout", &self.rel_unrounded_layout)
            .field("rel_final_layout", &self.rel_final_layout)
            .field("abs_layout", &self.abs_layout)
            .field("cache", &self.cache)
            .field("children", &self.children)
            .finish()
    }
}

impl Node for Element {
    #[inline(always)]
    fn render(&mut self, layout: &Layout) -> Option<Primitive> {
        self.inner
            .render(layout, &maybe_get_clone_untracked(&self.style).into())
    }
    #[inline(always)]
    fn measure(
        &mut self,
        measure_ctx: &mut dyn MeasureCtx,
        known_dimensions: Size<Option<f32>>,
        available_space: Size<AvailableSpace>,
        style: &taffy::Style,
    ) -> Size<f32> {
        self.inner
            .measure(measure_ctx, known_dimensions, available_space, style)
    }
    #[inline(always)]
    fn debug_label(&self) -> &'static str {
        self.inner.debug_label()
    }

    fn children(&self) -> &[ElementId] {
        &self.children
    }

    #[inline(always)]
    fn mouse_event(&mut self, ctx: &mut EventContext<MouseEvent>) {
        self.mouse_handler.handle(self, ctx);
        if !ctx.is_preventing_default() {
            self.inner.default_mouse_event(ctx, &self.abs_layout);
        }
    }
    #[inline(always)]
    fn keyboard_event(&mut self, ctx: &mut EventContext<KeyboardEvent>) {
        self.keyboard_handler.handle(self, ctx);
        if !ctx.is_preventing_default() {
            self.inner.default_keyboard_event(ctx, &self.abs_layout);
        }
    }

    #[inline(always)]
    fn focus_event(&mut self, ctx: &mut EventContext<FocusEvent>) {
        self.focus_handler.handle(self, ctx);
        if !ctx.is_preventing_default() {
            self.inner.default_focus_event(ctx, &self.abs_layout);
        }
    }
    #[inline(always)]
    fn blur_event(&mut self, ctx: &mut EventContext<BlurEvent>) {
        self.blur_handler.handle(self, ctx);
        if !ctx.is_preventing_default() {
            self.inner.default_blur_event(ctx, &self.abs_layout);
        }
    }

    fn get_zindex_properties(&self) -> ZIndexProperties {
        maybe_get_untracked(&self.zindex)
    }

    #[inline(always)]
    fn get_style(&self) -> taffy::Style {
        maybe_get_clone_untracked(&self.style).into()
    }

    #[inline(always)]
    fn get_abs_layout(&self) -> &taffy::Layout {
        &self.abs_layout
    }

    #[inline(always)]
    fn get_relative_unrounded_layout(&self) -> &taffy::Layout {
        &self.rel_unrounded_layout
    }
    #[inline(always)]
    fn get_relative_final_layout(&self) -> &taffy::Layout {
        &self.rel_final_layout
    }

    #[inline(always)]
    fn set_relative_unrounded_layout(&mut self, layout: Layout) {
        self.rel_unrounded_layout = layout;
    }
    #[inline(always)]
    fn set_relative_final_layout(&mut self, layout: taffy::Layout) {
        self.rel_final_layout = layout;
    }
    #[inline(always)]
    fn set_abs_layout(&mut self, layout: taffy::Layout) {
        self.abs_layout = layout;
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
    fn parent_id(&self) -> Option<ElementId> {
        self.parent_id
    }
}

use std::{cell::Cell, rc::Rc, sync::Arc};

use input::{KeyboardEvent, MouseEvent};
use slotmap::SlotMap;
use sycamore_reactive::{MaybeDyn, create_effect};
use taffy::Layout;

use crate::{
    ElementId,
    TreeManager,
    events::{BlurEvent, EventHandler, FocusEvent},
    tree::{Element, MaybeDynStyle, StyleWrapper, Widget},
    zindex::ZIndexProperties,
};

use crate::widgets::reactivity::ReactiveChildren;

pub trait ErasedBuilder {
    fn build(
        self: Box<Self>,
        arena: &mut SlotMap<ElementId, Element>,
        parent_id: Option<ElementId>,
    ) -> ElementId;

    fn into_box_dyn(self) -> Box<dyn ErasedBuilder>
    where
        Self: Sized + 'static,
    {
        Box::new(self) as Box<dyn ErasedBuilder>
    }
}

pub struct ElementBuilder<W: Widget> {
    inner: W,
    style: MaybeDyn<StyleWrapper>,
    zindex: MaybeDyn<ZIndexProperties>,

    children: Vec<Box<dyn ErasedBuilder>>,

    pub(crate) mouse_handler: EventHandler<MouseEvent>,
    pub(crate) keyboard_handler: EventHandler<KeyboardEvent>,
    pub(crate) focus_handler: EventHandler<FocusEvent>,
    pub(crate) blur_handler: EventHandler<BlurEvent>,

    #[allow(clippy::type_complexity)]
    pub(crate) before_build: Option<Arc<dyn Fn(&W)>>,
    pub(crate) after_build: Option<Arc<dyn Fn(ElementId)>>,
}

impl<W: Widget> ElementBuilder<W> {
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            style: MaybeDyn::Static(StyleWrapper::default()),
            zindex: MaybeDyn::Static(ZIndexProperties::DEFAULT),

            children: Vec::new(),

            mouse_handler: EventHandler::empty(),
            keyboard_handler: EventHandler::empty(),
            focus_handler: EventHandler::empty(),
            blur_handler: EventHandler::empty(),

            before_build: None,
            after_build: None,
        }
    }

    pub fn new_with_before_build<F>(inner: W, before_build: F) -> Self
    where
        F: Fn(&W) + 'static,
    {
        let mut builder = Self::new(inner);
        builder.before_build = Some(Arc::new(before_build) as Arc<dyn Fn(&W)>);
        builder
    }

    pub fn append_before_build<F>(mut self, before_build: F) -> Self
    where
        F: Fn(&W) + 'static,
        W: 'static,
    {
        self.before_build = if let Some(prev_build) = self.before_build {
            Some(Arc::new(move |inner: &W| {
                (prev_build)(inner);
                (before_build)(inner);
            }) as Arc<dyn Fn(&W)>)
        } else {
            Some(Arc::new(before_build) as Arc<dyn Fn(&W)>)
        };
        self
    }

    pub fn replace_before_build<F>(mut self, before_build: F) -> Self
    where
        F: Fn(&W) + 'static,
    {
        self.before_build = Some(Arc::new(before_build) as Arc<dyn Fn(&W)>);
        self
    }

    pub fn new_with_after_build<F>(inner: W, after_build: F) -> Self
    where
        F: Fn(ElementId) + 'static,
    {
        let mut builder = Self::new(inner);
        builder.after_build = Some(Arc::new(after_build) as Arc<dyn Fn(ElementId)>);
        builder
    }
    pub fn append_after_build<F>(mut self, after_build: F) -> Self
    where
        F: Fn(ElementId) + 'static,
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

    pub fn inner(&self) -> &W {
        &self.inner
    }
    pub fn inner_mut(&mut self) -> &mut W {
        &mut self.inner
    }
    pub fn into_inner(self) -> W {
        self.inner
    }
}

impl<W: Widget> ElementBuilder<W> {
    pub fn child(mut self, child: impl Into<BuilderList>) -> Self {
        self.children.extend(child.into().0);
        self
    }

    /// Add reactive children that are dynamically created/removed based on a closure.
    ///
    /// The closure is called reactively, and whenever its output changes, children are
    /// automatically added or removed. Each child gets its own reactive scope that is
    /// properly disposed when the child is removed.
    ///
    /// # Example
    /// ```rust,ignore
    /// let items = create_signal(vec!["a", "b", "c"]);
    /// div()
    ///     .reactive_child(|| {
    ///         items.get().into_iter().map(|item| {
    ///             div().child(text(item))
    ///         }).collect::<Vec<_>>()
    ///     })
    /// ```
    pub fn reactive_child<F>(mut self, f: F) -> Self
    where
        F: Fn() -> Vec<Box<dyn ErasedBuilder>> + 'static,
    {
        let closure_for_effect = Rc::new(f);

        // Set up the reactive effect in after_build
        // We append to existing after_build if it exists
        let existing_after_build = self.after_build.take();
        self.after_build = Some(Arc::new(move |parent_id| {
            // Call existing after_build if any
            if let Some(ref existing) = existing_after_build {
                existing(parent_id);
            }

            // Set up reactive children management
            let reactive_children = ReactiveChildren::new(
                {
                    let closure = closure_for_effect.clone();
                    move || closure()
                },
                parent_id,
            );

            reactive_children.setup_effect();
        }));

        self
    }

    /// Assign a style to this element.
    ///
    /// ```rust
    /// div().style(Style::DEFAULT);
    /// ```
    pub fn style(mut self, style: impl Into<MaybeDynStyle>) -> Self {
        let style = style.into().get();
        self.style = style.clone();
        self
    }

    /// Assign a z index properties to this element.
    ///
    /// ```rust
    /// div().zindex(ZIndexProperties::DEFAULT);
    /// ```
    pub fn zindex(mut self, zindex: impl Into<MaybeDyn<ZIndexProperties>>) -> Self {
        let zindex = zindex.into();
        self.zindex = zindex.clone();
        self
    }
}
impl<W: Widget + 'static> ElementBuilder<W> {
    #[profiling::function]
    pub(crate) fn build(
        self,
        arena: &mut SlotMap<ElementId, Element>,
        parent_id: Option<ElementId>,
    ) -> ElementId {
        if let Some(hook) = self.before_build {
            hook(&self.inner);
        }

        let key = arena.insert_with_key(|id| Element {
            node_id: id,
            parent_id,

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
            let child_id = child.build(arena, Some(key));
            arena[key].children.push(child_id);
        }

        // Post-build hooks
        if let Some(hook) = self.after_build {
            hook(key);
        }

        // effect hooks
        create_effect({
            let first_run = Cell::new(true);
            let mgr = TreeManager::global();
            let style = arena[key].style.clone();

            let mut last_display = style.get_clone().0.display;
            move || {
                let new_display = style.get_clone().0.display;
                if !first_run.get() {
                    log::trace!("node style changed: relayouting {key:?}");
                    if new_display != last_display {
                        // Display changes alter the render hierarchy (hiding/showing subtrees).
                        // We must rebuild the render order.
                        mgr.mark_structure_dirty();
                        last_display = new_display;
                    }
                    mgr.mark_layout_dirty(key);
                    mgr.now();
                } else {
                    first_run.set(false);
                }
            }
        });
        create_effect({
            let first_run = Cell::new(true);
            let mgr = TreeManager::global();
            let zindex = arena[key].zindex.clone();
            move || {
                zindex.track();
                if !first_run.get() {
                    mgr.mark_structure_dirty();
                    mgr.now();
                } else {
                    first_run.set(false);
                }
            }
        });

        key
    }
}

impl<W: Widget + 'static> ErasedBuilder for ElementBuilder<W> {
    fn build(
        self: Box<Self>,
        arena: &mut SlotMap<ElementId, Element>,
        parent_id: Option<ElementId>,
    ) -> ElementId {
        (*self).build(arena, parent_id)
    }
}

pub struct BuilderList(Vec<Box<dyn ErasedBuilder>>);

impl<T: ErasedBuilder + 'static> From<T> for BuilderList {
    fn from(val: T) -> Self {
        BuilderList(vec![Box::new(val)])
    }
}

impl<T: ErasedBuilder + 'static> From<Option<T>> for BuilderList {
    fn from(val: Option<T>) -> Self {
        if let Some(val) = val {
            BuilderList(vec![Box::new(val)])
        } else {
            BuilderList(vec![])
        }
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

impl From<Option<Box<dyn ErasedBuilder>>> for BuilderList {
    fn from(val: Option<Box<dyn ErasedBuilder>>) -> Self {
        if let Some(val) = val {
            BuilderList(vec![val])
        } else {
            BuilderList(vec![])
        }
    }
}
impl From<Box<dyn ErasedBuilder>> for BuilderList {
    fn from(val: Box<dyn ErasedBuilder>) -> Self {
        BuilderList(vec![val])
    }
}
impl From<Vec<Box<dyn ErasedBuilder>>> for BuilderList {
    fn from(val: Vec<Box<dyn ErasedBuilder>>) -> Self {
        BuilderList(val)
    }
}

impl<T: ErasedBuilder + 'static, const N: usize> From<[T; N]> for BuilderList {
    fn from(val: [T; N]) -> Self {
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
    (
        A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U
    ),
    (
        A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V
    ),
    (
        A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W
    ),
    (
        A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X
    ),
    (
        A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y
    ),
    (
        A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z
    )
];

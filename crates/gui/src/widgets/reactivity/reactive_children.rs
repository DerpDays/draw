use std::{cell::Cell, rc::Rc};

use graphics::Primitive;
use sycamore_reactive::{NodeHandle, create_child_scope, create_effect};
use taffy::{AvailableSpace, Layout, Size};

use crate::{
    ElementId,
    MeasureCtx,
    TreeManager,
    tree::{
        Widget,
        builder::{BuilderList, ElementBuilder},
    },
};

impl Widget for ReactiveChildren {
    fn render(&mut self, _layout: &Layout, _style: &taffy::Style) -> Option<Primitive> {
        None
    }

    fn measure(
        &mut self,
        _: ElementId,
        _: &mut dyn MeasureCtx,
        known_dimensions: Size<Option<f32>>,
        _available: Size<AvailableSpace>,
        _style: &taffy::Style,
    ) -> Size<f32> {
        known_dimensions.unwrap_or(Size::zero())
    }

    fn debug_label(&self) -> &'static str {
        "Reactive Children"
    }

    fn focusable(&self) -> bool {
        false
    }
}

pub struct ReactiveChildren {
    scope: Rc<Cell<NodeHandle>>,
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
/// div().child(
///     reactive(|| {
///         items.get().into_iter().map(|item| {
///             div().child(text(item))
///         }).collect::<Vec<_>>()
///     })
/// )
/// ```
pub fn reactive<F>(f: F) -> ElementBuilder<ReactiveChildren>
where
    F: Fn() -> BuilderList + 'static + Clone,
{
    let scope = Rc::new(Cell::new(create_child_scope(|| {})));
    let current_scope = sycamore_reactive::use_current_scope();

    ElementBuilder::new(
        ReactiveChildren {
            scope: scope.clone(),
        },
        // |inner| inner.scope.set(Some(create_child_scope(|| {}))),
    )
    .append_after_build(move |elem_id| {
        let mgr = TreeManager::global();
        let f = f.clone();
        let scope = scope.clone();
        create_effect(move || {
            let new_scope = current_scope.run_in(|| create_child_scope(|| {}));
            let old_scope = scope.replace(new_scope);
            // .expect("we always have a scope set for a reactive element");

            let children_builders = new_scope.run_in(|| f().0);
            mgr.queue_replace_children(elem_id, children_builders, old_scope, new_scope);
        })
    })
}

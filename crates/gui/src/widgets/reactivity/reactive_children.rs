use std::{cell::RefCell, rc::Rc};

use graphics::Primitive;
use sycamore_reactive::{NodeHandle, create_child_scope, create_effect};
use taffy::{AvailableSpace, Layout, Size};

use crate::{
    ElementId,
    MeasureCtx,
    TreeManager,
    tree::{Widget, builder::ErasedBuilder},
};

pub struct ReactiveChildren {
    closure: Rc<dyn Fn() -> Vec<Box<dyn ErasedBuilder>>>,
    parent_id: ElementId,
    // Store (child_id, scope_handle) pairs
    current_children: Rc<RefCell<Vec<(ElementId, NodeHandle)>>>,
}

impl ReactiveChildren {
    pub fn new<F>(closure: F, parent_id: ElementId) -> Self
    where
        F: Fn() -> Vec<Box<dyn ErasedBuilder>> + 'static,
    {
        Self {
            closure: Rc::new(closure),
            parent_id,
            current_children: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn setup_effect(&self) {
        let closure = self.closure.clone();
        let parent_id = self.parent_id;
        let current_children = self.current_children.clone();
        let mgr = TreeManager::global();

        create_effect(move || {
            // Call the closure to get new children
            let new_builders = closure();

            let mut children_guard = current_children.borrow_mut();
            let current_len = children_guard.len();
            let new_len = new_builders.len();

            // Simple diff: if length changed, remove all and rebuild
            // This can be optimized later with proper key-based diffing
            if current_len != new_len {
                // Dispose all existing scopes and remove children
                // We need to dispose scopes in the reactive context (here),
                // then queue the removal
                let child_ids_to_remove: Vec<ElementId> = children_guard
                    .drain(..)
                    .map(|(child_id, scope_handle)| {
                        // Dispose the scope first to clean up effects
                        log::trace!("disposing of reactive child {child_id:?}");
                        scope_handle.dispose();
                        child_id
                    })
                    .collect();

                // Queue removal of all children (scopes already disposed)
                for child_id in child_ids_to_remove {
                    mgr.queue_remove_child(parent_id, child_id, None::<fn()>);
                }

                // Add new children, each with its own scope
                for builder in new_builders {
                    let children_for_callback = current_children.clone();

                    // Create a scope for this child
                    // The scope closure runs immediately, and any effects created
                    // within it will be scoped to this handle
                    let scope_handle = create_child_scope(|| {
                        // Empty - we just need the scope to exist
                        // The scope handle will be used to track this child's lifecycle
                    });

                    // Queue adding the child with a callback
                    // The callback will store the child_id -> scope_handle mapping
                    mgr.queue_add_child(
                        parent_id,
                        builder,
                        Some(move |child_id| {
                            // Callback to store the mapping when child is added
                            let mut children = children_for_callback.borrow_mut();
                            children.push((child_id, scope_handle));
                        }),
                    );
                }
            }

            mgr.now();
        });
    }

    pub fn add_child_mapping(&self, child_id: ElementId, scope_handle: NodeHandle) {
        let mut children = self.current_children.borrow_mut();
        children.push((child_id, scope_handle));
    }
}

impl Widget for ReactiveChildren {
    fn render(&mut self, _layout: &Layout, _style: &taffy::Style) -> Option<Primitive> {
        // ReactiveChildren is a container widget, it doesn't render itself
        None
    }

    fn measure(
        &mut self,
        _: &mut dyn MeasureCtx,
        known_dimensions: Size<Option<f32>>,
        _available: Size<AvailableSpace>,
        _style: &taffy::Style,
    ) -> Size<f32> {
        known_dimensions.unwrap_or(Size::zero())
    }

    fn debug_label(&self) -> &'static str {
        "ReactiveChildren"
    }

    fn focusable(&self) -> bool {
        false
    }
}

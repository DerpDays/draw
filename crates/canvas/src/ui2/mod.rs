use std::{cell::OnceCell, rc::Rc, sync::OnceLock};

use euclid::default::Point2D;
use graphics::Systems;
use gui_reactive::{
    prelude::*,
    reexports::{
        reactive::{create_signal, Signal},
        taffy,
    },
    tree::{builder::ErasedBuilder, Node, StyleWrapper},
    widgets::primitives::div,
    Tree,
    TreeManager,
};
use input::{Modifiers, MouseButton, MouseEvent, MouseEventKind};
use renderer::GrowableMeshBuffer;

use crate::{tools::ToolKind, RedrawRequest, RedrawRequestV2};

mod options;
mod styles;
mod toolbar;

pub struct Application {
    pub tree: gui_reactive::Tree,
    pub gui_buffer: GrowableMeshBuffer,

    selected_tool: Signal<ToolKind>,

    pub modifiers: Modifiers,
}

impl Application {
    pub fn new<T: RedrawRequestV2 + Clone + Send + Sync + 'static>(
        gui_buffer: GrowableMeshBuffer,
        redraw_manager: &T,
    ) -> Self {
        let tree_manager = TreeManager::new(
            {
                let redraw_manager = redraw_manager.clone();
                move || redraw_manager.request_redraw()
            },
            {
                let redraw_manager = redraw_manager.clone();
                move || redraw_manager.new_animation_handle()
            },
            // {
            //     let redraw_manager = redraw_manager.clone();
            //     move |duration| redraw_manager.request_redraw_duration(duration)
            // },
        );

        let selected_tool_slot: Rc<OnceCell<Signal<ToolKind>>> = Rc::new(OnceCell::new());
        let selected_tool_slot_clone = selected_tool_slot.clone();

        let tree = Tree::build(taffy::Size::MAX_CONTENT, tree_manager, move || {
            let selected_tool = create_signal(ToolKind::default());
            selected_tool_slot_clone
                .set(selected_tool)
                .expect("selected tool signal should be initialised once");

            div().child(app(selected_tool))
        });

        Self {
            tree,
            gui_buffer,
            selected_tool: *selected_tool_slot
                .get()
                .expect("selected tool signal has not been initialised"),
            modifiers: Modifiers::empty(),
        }
    }

    pub fn render(&mut self, systems: &mut Systems) {
        let rendered = self.tree.render(systems);
        _ = self
            .gui_buffer
            .replace_with_mesh(&systems.device, &systems.queue, &rendered);
    }

    pub fn selected_tool(&self) -> ToolKind {
        self.tree
            .owner
            .run_in(|| self.selected_tool.get_untracked())
    }
}

pub fn floating_grab(grab_area: f32, top_left: Point2D<f32>) -> StyleWrapper {
    Style {
        display: Display::Grid,
        position: Position::Absolute,
        inset: Rect {
            left: LengthPercentageAuto::length(top_left.x),
            top: LengthPercentageAuto::length(top_left.y),
            right: LengthPercentageAuto::AUTO,
            bottom: LengthPercentageAuto::AUTO,
        },
        padding: Rect::<LengthPercentage>::length(grab_area),
        grid_template_rows: vec![GridTemplateComponent::AUTO],
        grid_template_columns: vec![GridTemplateComponent::AUTO],
        ..Default::default()
    }
    .into()
}
pub fn drag_fn(
    state: Signal<Option<DragState>>,
    style: Signal<StyleWrapper>,
) -> impl Fn(&dyn Node, &mut EventContext<MouseEvent>) {
    move |elem, ctx| match ctx.current_phase() {
        EventPhase::Bubbling | EventPhase::AtTarget | EventPhase::Direct => {
            match ctx.payload().kind {
                MouseEventKind::Enter | MouseEventKind::Leave => {
                    state.set(None);
                }
                MouseEventKind::Press { button, .. } if button == MouseButton::Left => {
                    let layout = elem.get_final_layout();
                    state.set(Some(DragState {
                        origin: Point2D::new(layout.location.x, layout.location.y),
                        start: ctx.payload().position,
                    }));
                    ctx.request_mouse_capture(ctx.current_node());
                }
                MouseEventKind::Release { .. } => {
                    state.set(None);
                    ctx.request_mouse_release();
                }
                MouseEventKind::Motion { .. } => {
                    if let Some(drag) = state.get_untracked() {
                        let new_origin =
                            (drag.origin + (ctx.payload().position - drag.start)).round();
                        tracing::info!("updating position!!! {new_origin:#?}");
                        style.set(floating_grab(100., new_origin));
                    } else {
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DragState {
    /// Where to base the movement from.
    origin: Point2D<f32>,
    /// Where to calculate the change in position from.
    start: Point2D<f32>,
}

fn app(selected_tool: Signal<ToolKind>) -> impl ErasedBuilder {
    // root elem
    div().child((toolbar::toolbar(selected_tool), options::options()))
}

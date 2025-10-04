use color::AlphaColor;
use euclid::default::Point2D;
use graphics::Systems;
use gui::reexports::taffy::Style;
use gui_reactive::{
    prelude::*,
    reexports::taffy,
    tree::{ErasedBuilder, StyleWrapper},
    widgets::primitives::div,
    Tree, TreeManager,
};
use input::Modifiers;
use renderer::GrowableMeshBuffer;

use crate::RedrawRequest;

mod options;
mod styles;
mod toolbar;

pub struct Application {
    pub tree: gui_reactive::Tree,
    pub gui_buffer: GrowableMeshBuffer,

    pub modifiers: Modifiers,
}

impl Application {
    pub fn new<T: RedrawRequest + Clone + Send + Sync + 'static>(
        gui_buffer: GrowableMeshBuffer,
        redraw_manager: &T,
    ) -> Self {
        let redraw_manager_1 = redraw_manager.clone();
        let redraw_manager_2 = redraw_manager.clone();
        let tree_manager = TreeManager::new(
            move || redraw_manager_1.request_redraw(),
            move |duration| redraw_manager_2.request_redraw_duration(duration),
        );
        let tree = Tree::build(taffy::Size::MAX_CONTENT, tree_manager, || {
            div().child(app())
        });
        tree.owner.set();

        Self {
            tree,
            gui_buffer,
            modifiers: Modifiers::empty(),
        }
    }

    pub fn render(&mut self, systems: &mut Systems) {
        let rendered = self.tree.render(systems);
        _ = self
            .gui_buffer
            .replace_with_mesh(&systems.device, &systems.queue, &rendered);
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

#[derive(Clone, Copy, Debug)]
pub struct DragState {
    /// Where to base the movement from.
    origin: Point2D<f32>,
    /// Where to calculate the change in position from.
    start: Point2D<f32>,
}

fn app() -> impl ErasedBuilder {
    // root elem
    div().child((toolbar::toolbar(), options::options()))
}

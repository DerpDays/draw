use graphics::{Mesh, Systems, Vertex};
use reactive_graph::{
    effect::Effect,
    traits::{Get, GetUntracked},
    wrappers::read::Signal,
};
use taffy::{AvailableSpace, Layout, Size, Style};

use crate::{
    TreeManager,
    tree::{Element, ElementBuilder, Widget},
};

pub struct Text {
    pub text: Signal<String>,
}
impl Widget for Text {
    fn render(
        &mut self,
        _mesh: &mut Mesh<Vertex>,
        _sys: &mut Systems,
        _layout: &Layout,
        _: &Style,
    ) {
        println!("Render Label: {}", self.text.get_untracked());
    }
    fn measure(
        &mut self,
        known_dimensions: Size<Option<f32>>,
        _: Size<AvailableSpace>,
        _: &Style,
    ) -> Size<f32> {
        // TODO: actually measure text
        known_dimensions.unwrap_or(Size::zero())
    }
    fn debug_label(&self) -> &'static str {
        "Text"
    }

    fn focusable(&self) -> bool {
        false
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self as &dyn std::any::Any
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self as &mut dyn std::any::Any
    }
}

pub fn text(text: impl Into<Signal<String>>) -> ElementBuilder<Text> {
    let text = text.into();
    ElementBuilder::new_with_callback(Text { text }, move |elem_id| {
        let mgr = TreeManager::global();

        Effect::watch_sync(
            move || text.get(),
            move |new, old, _| {
                if Some(new) != old {
                    tracing::debug!("new text!!");
                    mgr.relayout(elem_id);
                    mgr.now();
                }
            },
            false,
        );
    })
}

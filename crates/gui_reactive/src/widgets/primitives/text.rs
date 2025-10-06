use graphics::{Mesh, Systems, Vertex};
use sycamore_reactive::{ReadSignal, create_effect};
use taffy::{AvailableSpace, Layout, Size, Style};

use crate::{
    TreeManager,
    tree::{Widget, builder::ElementBuilder},
};

pub struct Text {
    pub text: ReadSignal<String>,
}
impl Widget for Text {
    fn render(
        &mut self,
        _mesh: &mut Mesh<Vertex>,
        _sys: &mut Systems,
        _layout: &Layout,
        _: &Style,
    ) {
        println!("Render Label: {}", self.text.get_clone_untracked());
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
}

pub fn text(text: ReadSignal<String>) -> ElementBuilder<Text> {
    let text = text.into();
    ElementBuilder::new_with_after_build(Text { text }, move |elem_id| {
        let mgr = TreeManager::global();

        create_effect(move || {
            text.track();
            tracing::debug!("new text!!");
            mgr.relayout(elem_id);
            mgr.now();
        });
    })
}

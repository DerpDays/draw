use graphics::{Mesh, Systems, Vertex};
use reactive_graph::{
    effect::Effect,
    traits::{Get, GetUntracked},
    wrappers::read::Signal,
};
use taffy::{AvailableSpace, Layout, Size, Style};

use crate::{
    tree::{DynNode, Element, ElementWithChildren, Widget},
    TreeManager,
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

    fn focusable() -> bool {
        false
    }
}
impl ElementWithChildren for Text {}

pub fn text(text: impl Into<Signal<String>>) -> Element<Text, ()> {
    let text = text.into();
    let elem = Element::new_empty(Text { text });
    let node_id = elem.node_id();
    let mgr = TreeManager::global();

    Effect::watch_sync(
        move || text.get(),
        move |new, old, _| {
            if Some(new) != old {
                mgr.relayout(node_id);
                mgr.now();
            }
        },
        false,
    );
    elem
}

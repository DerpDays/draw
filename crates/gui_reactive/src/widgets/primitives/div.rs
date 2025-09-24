use graphics::{Mesh, Systems, Vertex};
use taffy::{AvailableSpace, Layout, Size, Style};

use crate::tree::{Element, ElementWithChildren, Widget};

pub struct Div;
impl ElementWithChildren for Div {}
impl Widget for Div {
    fn render(&mut self, _mesh: &mut Mesh<Vertex>, _sys: &mut Systems, _layout: &Layout) {
        println!("render div")
    }
    fn measure(
        &mut self,
        known_dimensions: Size<Option<f32>>,
        _: Size<AvailableSpace>,
        _: &Style,
    ) -> Size<f32> {
        known_dimensions.unwrap_or(Size::zero())
    }
    fn debug_label(&self) -> &'static str {
        "Div"
    }

    fn focusable() -> bool {
        false
    }
}

pub fn div() -> Element<Div, ()> {
    Element::new_empty(Div)
}

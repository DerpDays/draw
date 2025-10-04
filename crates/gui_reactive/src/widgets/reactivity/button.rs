use graphics::{Mesh, Systems, Vertex};
use taffy::{AvailableSpace, Layout, Size, Style};

use crate::tree::{Element, Widget};

pub struct Button;
impl Widget for Button {
    fn render(
        &mut self,
        _mesh: &mut Mesh<Vertex>,
        _sys: &mut Systems,
        _layout: &Layout,
        _: &Style,
    ) {
        println!("Render button:");
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
        "Button"
    }

    fn focusable(&self) -> bool {
        true
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self as &dyn std::any::Any
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self as &mut dyn std::any::Any
    }
}

// pub fn button() -> Element<Button, ()> {
//     Element::new_empty(Button)
// }

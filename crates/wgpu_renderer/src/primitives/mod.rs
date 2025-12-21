mod rectangle;
pub use rectangle::render_rectangle;

mod ellipse;
pub use ellipse::render_ellipse;

mod text;
pub use text::{prepare_layout, render_text};

use graphics_v2::Primitive;

use crate::{GraphicsContext, Mesh, PrimitiveCache, vertex::Vertex};

pub trait Render {
    fn to_mesh(
        &self,
        ctx: &mut GraphicsContext<Vertex>,
        cache: &mut Option<PrimitiveCache>,
    ) -> Mesh<Vertex>;
}

#[allow(unused_variables)]
impl Render for Primitive {
    fn to_mesh(
        &self,
        ctx: &mut GraphicsContext<Vertex>,
        cache: &mut Option<PrimitiveCache>,
    ) -> Mesh<Vertex> {
        match self {
            Primitive::Ellipse(ellipse) => render_ellipse(ellipse),
            Primitive::Line(line) => todo!(),
            Primitive::CubicBezier(cubic_bezier) => todo!(),
            Primitive::Pen(pen) => todo!(),
            Primitive::Quad(quad) => todo!(),
            Primitive::Rectangle(rectangle) => render_rectangle(rectangle),
            Primitive::Svg(svg) => todo!(),
            Primitive::Text(text) => render_text(ctx, text, cache),
            Primitive::Triangle(triangle) => todo!(),
        }
    }
}

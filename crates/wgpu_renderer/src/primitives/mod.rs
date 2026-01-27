mod rectangle;

pub use rectangle::render_rectangle;

mod ellipse;
pub use ellipse::render_ellipse;

mod text;
pub use text::{prepare_text_layout, render_text};

// mod svg;
// pub use svg::render_svg;
mod svg_resvg;
pub use svg_resvg::render_svg;

use graphics::Primitive;

use crate::{
    GraphicsContext,
    Mesh,
    PrimitiveCache,
    shaders::{
        // basic_shape::BasicShapeVertex,
        generic,
        // text::TextVertex,
        texture::{TexturePrimitive, TextureVertex},
    },
};

#[derive(Copy, Clone, PartialEq)]
pub enum DrawType {
    Generic,
    Texture,
}

pub enum PrimitiveMesh {
    /// mesh, msaa
    Generic(Mesh<generic::Vertex>),
    // Text(Mesh<TextVertex>),
    Texture(([crate::shaders::texture::TextureVertex; 4], wgpu::BindGroup)),
}

pub trait PrimitiveToMesh {
    fn to_mesh(
        &self,
        ctx: &mut GraphicsContext,
        cache: &mut Option<PrimitiveCache>,
    ) -> PrimitiveMesh;
}
impl PrimitiveToMesh for Primitive {
    #[profiling::function]
    fn to_mesh(
        &self,
        ctx: &mut GraphicsContext,
        cache: &mut Option<PrimitiveCache>,
    ) -> PrimitiveMesh {
        match self {
            Primitive::Ellipse(ellipse) => todo!(),
            Primitive::Line(line) => todo!(),
            Primitive::CubicBezier(cubic_bezier) => todo!(),
            Primitive::Pen(pen) => todo!(),
            Primitive::Quad(quad) => todo!(),
            Primitive::Rectangle(rectangle) => PrimitiveMesh::Generic(render_rectangle(rectangle)),
            Primitive::Svg(svg) => PrimitiveMesh::Generic(render_svg(ctx, svg, cache)),
            Primitive::Text(text) => PrimitiveMesh::Generic(render_text(ctx, text, cache)),
            Primitive::Text(text) => todo!(),
            Primitive::Triangle(triangle) => todo!(),
            Primitive::Custom(custom) => {
                let Some(primitive) = custom.as_any().downcast_ref::<TexturePrimitive>() else {
                    panic!("tried to draw custom primitive that is not part of wgpu_renderer");
                };
                PrimitiveMesh::Texture((
                    TextureVertex::new_strip_quad(
                        primitive.area.min.to_array(),
                        primitive.area.max.to_array(),
                    ),
                    primitive.bind_group.clone().into(),
                ))
                // let Some(bind_group) =
            }
        }
    }
}

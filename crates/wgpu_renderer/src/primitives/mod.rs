mod rectangle;

pub use rectangle::render_rectangle;

mod ellipse;
pub use ellipse::render_ellipse;

mod text;
pub use text::{prepare_text_layout, render_text};

use graphics::Primitive;

use crate::{
    GraphicsContext,
    Mesh,
    PrimitiveCache,
    shaders::{
        basic_shape::BasicShapeVertex,
        text::TextVertex,
        texture::{TexturePrimitive, TextureVertex},
    },
};

#[derive(Copy, Clone, PartialEq)]
pub enum DrawType {
    BasicShape,
    BasicShapeMultisample,
    Text,
    Texture,
}

pub enum PrimitiveMesh {
    BasicShape(Mesh<BasicShapeVertex>),
    BasicShapeMultisample(Mesh<BasicShapeVertex>),
    Text(Mesh<TextVertex>),
    Texture(([crate::shaders::texture::TextureVertex; 4], wgpu::BindGroup)),
}
pub trait ToDrawType {
    fn to_draw_type(&self) -> DrawType;
}
impl ToDrawType for Primitive {
    #[inline(always)]
    #[profiling::function]
    fn to_draw_type(&self) -> DrawType {
        match self {
            Primitive::Svg(_) => DrawType::BasicShapeMultisample,
            Primitive::Text(_) => DrawType::Text,
            Primitive::Custom(custom) => {
                if let Some(_) = custom.as_any().downcast_ref::<TexturePrimitive>() {
                    DrawType::Texture
                } else {
                    panic!("tried to draw custom primitive that is not part of wgpu_renderer");
                }
            }
            _ => DrawType::BasicShape,
        }
    }
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
            Primitive::Rectangle(rectangle) => {
                PrimitiveMesh::BasicShape(render_rectangle(rectangle))
            }
            Primitive::Svg(svg) => todo!(),
            Primitive::Text(text) => PrimitiveMesh::Text(render_text(ctx, text, cache)),
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

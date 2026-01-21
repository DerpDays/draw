mod rectangle;
use std::intrinsics::unreachable;

pub use rectangle::render_rectangle;

mod ellipse;
pub use ellipse::render_ellipse;

mod text;
pub use text::{prepare_text_layout, render_text};

use graphics_v2::Primitive;

use crate::{GraphicsContext, Mesh, PrimitiveCache, vertex::Vertex};

#[derive(Copy, Clone, PartialEq)]
pub enum DrawType {
    BasicShape,
    BasicShapeMultisample,
    Text,
    Texture,
}
pub trait ToDrawType {
    fn to_draw_type(&self) -> DrawType;
}
impl ToDrawType for Primitive {
    #[inline(always)]
    fn to_draw_type(&self) -> DrawType {
        match self {
            Primitive::Svg(_) => DrawType::BasicShapeMultisample,
            Primitive::Text(_) => DrawType::Text,
            Primitive::Custom(custom) => {
                if let Some(_) = custom.as_any().downcast_ref::<wgpu::Texture>() {
                    DrawType::Texture
                } else {
                    panic!("tried to draw custom primitive that is not part of wgpu_renderer");
                }
            }
            _ => DrawType::BasicShape,
        }
    }
}

pub trait PrimitiveToBasicShapeMesh {
    fn basic_shape_mesh(
        &self,
        ctx: &mut GraphicsContext,
        cache: &mut Option<PrimitiveCache>,
    ) -> Mesh<crate::shaders::BasicShapeVertex> {
        unimplemented!("basic_shape_mesh not implemented for this primitive")
    }
}
impl PrimitiveToBasicShapeMesh for graphics_v2::primitives::Text {}

#[allow(unused_variables)]
pub trait PrimitiveToMesh {
    fn basic_shape_mesh(
        &self,
        ctx: &mut GraphicsContext,
        cache: &mut Option<PrimitiveCache>,
    ) -> Mesh<crate::shaders::BasicShapeVertex> {
        unimplemented!("basic_shape_mesh not implemented for this primitive")
    }
    fn text_mesh(
        &self,
        ctx: &mut GraphicsContext,
        cache: &mut Option<PrimitiveCache>,
    ) -> Mesh<crate::shaders::TextVertex> {
        unimplemented!("text_mesh not implemented for this primitive")
    }
    fn texture_mesh(
        &self,
        ctx: &mut GraphicsContext,
        cache: &mut Option<PrimitiveCache>,
    ) -> Mesh<crate::shaders::TextVertex> {
        unimplemented!("texture_mesh not implemented for this primitive")
    }
}

impl PrimitiveToMesh for Primitive {}

// #[allow(unused_variables)]
// impl Render for Primitive {
//     fn to_mesh(
//         &self,
//         ctx: &mut GraphicsContext,
//         cache: &mut Option<PrimitiveCache>,
//     ) -> Mesh<Vertex> {
//         profiling::function_scope!(format!("{self:?}").as_str());
//         match self {
//             Primitive::Ellipse(ellipse) => render_ellipse(ellipse),
//             Primitive::Line(line) => todo!(),
//             Primitive::CubicBezier(cubic_bezier) => todo!(),
//             Primitive::Pen(pen) => todo!(),
//             Primitive::Quad(quad) => todo!(),
//             Primitive::Rectangle(rectangle) => render_rectangle(rectangle),
//             Primitive::Svg(svg) => todo!(),
//             Primitive::Text(text) => render_text(ctx, text, cache),
//             Primitive::Triangle(triangle) => todo!(),
//             Primitive::Custom(custom) => todo!(),
//         }
//     }
// }

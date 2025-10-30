use std::collections::HashMap;

use euclid::default::Box2D;
use graphics_v2::{
    BasicColor,
    Primitive,
    make_positive_box,
    primitives::{Ellipse, Rectangle},
};
use gui_reactive::ElementId;

use crate::{GraphicsContext, Mesh, VertexArenaMarker, arena::Key, vertex::Vertex};

struct CacheEntry {
    previous_elem: graphics_v2::Primitive,
    vertex_alloc: Key<VertexArenaMarker>,
    indices: Vec<u32>,
}

struct GuiCache {
    hashmap: HashMap<ElementId, CacheEntry>,
}
impl GuiCache {
    pub fn update_elem(ctx: &mut GraphicsContext<Vertex>, elem: ElementId, primitive: Primitive) {
        ctx.vertex_buf.update(device, queue, key, data)
    }
}

pub trait Render {
    fn to_mesh(&self, ctx: &mut GraphicsContext<Vertex>) -> Mesh<Vertex> {}
}
impl Render for Primitive {
    fn to_mesh(&self, ctx: &mut GraphicsContext<Vertex>) -> Mesh<Vertex> {
        match self {
            Primitive::Ellipse(ellipse) => render_ellipse(ellipse),
            Primitive::Line(line) => todo!(),
            Primitive::CubicBezier(cubic_bezier) => todo!(),
            Primitive::Pen(pen) => todo!(),
            Primitive::Quad(quad) => todo!(),
            Primitive::Rectangle(rectangle) => render_rectangle(rectangle),
            Primitive::Svg(svg) => todo!(),
            Primitive::Text(text) => todo!(),
            Primitive::Triangle(triangle) => todo!(),
        }
    }
}

fn render_ellipse(ellipse: &Ellipse) -> Mesh<Vertex> {
    let mut buffers = VertexBuffers::<Vertex, u32>::new();
    let mut builder = BuffersBuilder::new(&mut buffers, |vertex: FillVertex<'_>| {
        Vertex::with_color(
            vertex.position(),
            C::apply(VertexKind::Color(self.options.color)),
        )
    });

    let options = FillOptions::tolerance(0.1);
    let mut tessellator = FillTessellator::new();

    let tessellation_result = tessellator.tessellate_path(&self.path, &options, &mut builder);
    if let Err(err) = tessellation_result {
        warn!(
            "Error while tessellating ellipse with options {:?}: {}",
            ellipse, err
        );
    }

    Mesh {
        vertices: buffers.vertices,
        indices: buffers.indices,
    }
}

fn render_rectangle(rect: &Rectangle) -> Mesh<Vertex> {
    let area = make_positive_box(Box2D::from_origin_and_size(rect.origin, rect.size));
    // does not require tesselation, making it cheap to generate
    if rect.rounding.is_zero() {
        let mesh = if rect.stroke_width != 0 {
            let vertices = Vec::with_capacity(8);
            let indices = Vec::with_capacity(8);
            Mesh { vertices, indices }
        } else {
            Mesh {
                vertices: Vec::with_capacity(8),
                indices: Vec::with_capacity(8),
            }
        };
        match rect.color {
            BasicColor::Solid(alpha_color) => {
                mesh.vertices
                    .push(Vertex::new_solid_rect(area.min, area.max, rect.color));
            }
            BasicColor::LinearGradient(basic_linear_gradient) => todo!(),
        }
        mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
        mesh.vertices
            .push(Vertex::new_solid_rect(area.min, area.max, rect.color));
        mesh
    } else {
        Mesh { vertices, indices }
    }
}

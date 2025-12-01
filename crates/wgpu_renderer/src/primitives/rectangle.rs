use euclid::default::{Box2D, SideOffsets2D};
use graphics_v2::{make_positive_box, primitives::Rectangle, BasicColor, BasicLinearGradient};
use lyon::{
    path::{Path, Winding},
    tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers},
};

use crate::{Mesh, Vertex};

pub fn render_rectangle(rect: &Rectangle) -> Mesh<Vertex> {
    if !rect.rounding.is_zero() {
        return render_rounded_rectangle(rect);
    }

    let area = make_positive_box(Box2D::from_origin_and_size(rect.origin, rect.size));

    // does not require tesselation, making it cheap to generate
    if rect.stroke_width != 0. {
        let mut vertices = Vec::with_capacity(8);
        let mut indices = Vec::with_capacity(12);
        basic_quad(
            area.outer_box(SideOffsets2D::new_all_same(rect.stroke_width)),
            &rect.color,
            &mut vertices,
        );
        indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
        basic_quad(area, &rect.color, &mut vertices);
        indices.extend_from_slice(&[4, 5, 6, 4, 6, 7]);
        Mesh { vertices, indices }
    } else {
        let mut vertices = Vec::with_capacity(4);
        let mut indices = Vec::with_capacity(6);
        basic_quad(area, &rect.color, &mut vertices);
        indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
        Mesh { vertices, indices }
    }
}

#[inline(always)]
fn basic_quad(area: Box2D<f32>, color: &BasicColor, vertices: &mut Vec<Vertex>) {
    vertices.extend_from_slice(&match color {
        BasicColor::Solid(color) => Vertex::new_solid_rect(
            area.min.to_array(),
            area.max.to_array(),
            color.convert().premultiply(),
        ),
        BasicColor::LinearGradient(gradient) => {
            Vertex::new_gradient_rect(area.min.to_array(), area.max.to_array(), gradient)
        }
    });
}

#[inline(always)]
// WARN: This is wrong for translucent fill's which contain a stroke color;
fn render_rounded_rectangle(rect: &Rectangle) -> Mesh<Vertex> {
    // We can receive negative areas in Box2D since size can also be negative,
    // therefore we need to change ensure each axis are their actual minimum/maximum.
    let area = make_positive_box(Box2D::from_origin_and_size(rect.origin, rect.size));

    let mut fill_path = Path::builder();
    let mut stroke_path = Path::builder();

    if rect.stroke_width == 0. {
        fill_path.add_rounded_rectangle(&area, &rect.rounding.to_lyon(), Winding::Positive);
    } else {
        let inner = area.inner_box(euclid::SideOffsets2D::new_all_same(rect.stroke_width));

        stroke_path.add_rounded_rectangle(&area, &rect.rounding.to_lyon(), Winding::Positive);
        if !inner.is_empty() {
            fill_path.add_rounded_rectangle(&inner, &rect.rounding.to_lyon(), Winding::Positive);
        }
    }
    let fill_path = fill_path.build();
    let stroke_path = stroke_path.build();

    let mut buffers = VertexBuffers::<Vertex, u32>::new();
    let options = FillOptions::default();
    let mut tessellator = FillTessellator::new();

    let mut stroke_builder = BuffersBuilder::new(&mut buffers, as_vertex_fn(rect.stroke_color));
    _ = tessellator.tessellate_path(&stroke_path, &options, &mut stroke_builder);

    let mut fill_builder = BuffersBuilder::new(&mut buffers, as_vertex_fn(rect.color));
    _ = tessellator.tessellate_path(&fill_path, &options, &mut fill_builder);

    Mesh {
        vertices: buffers.vertices,
        indices: buffers.indices,
    }
}

fn as_vertex_fn(color: BasicColor) -> impl Fn(FillVertex<'_>) -> Vertex {
    move |vertex: FillVertex<'_>| {
        Vertex::new_color(
            vertex.position().to_array(),
            match color {
                BasicColor::Solid(color) => color.convert().premultiply(),
                BasicColor::LinearGradient(gradient) => gradient
                    .get_point(vertex.position())
                    .convert()
                    .premultiply(),
            },
        )
    }
}

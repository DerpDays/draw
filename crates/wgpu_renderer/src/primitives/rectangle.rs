use euclid::default::{Box2D, SideOffsets2D};
use graphics_v2::{BasicColor, make_positive_box, primitives::Rectangle};
use lyon::{
    path::{Path, Winding},
    tessellation::{
        BuffersBuilder,
        FillOptions,
        FillTessellator,
        FillVertex,
        StrokeOptions,
        StrokeTessellator,
        StrokeVertex,
        VertexBuffers,
    },
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
fn render_rounded_rectangle(rect: &Rectangle) -> Mesh<Vertex> {
    // We can receive negative areas in Box2D since size can also be negative,
    // therefore we need to change ensure each axis are their actual minimum/maximum.
    let area = make_positive_box(Box2D::from_origin_and_size(rect.origin, rect.size));

    let mut fill_path = Path::builder();
    let mut stroke_path = Path::builder();
    let fill_rect = area.inner_box(SideOffsets2D::new_all_same(rect.stroke_width));
    if !fill_rect.is_empty() {
        fill_path.add_rounded_rectangle(&fill_rect, &rect.rounding.to_lyon(), Winding::Positive);
    }

    let stroke_rect = area.inner_box(SideOffsets2D::new_all_same(rect.stroke_width / 2.));
    stroke_path.add_rounded_rectangle(&stroke_rect, &rect.rounding.to_lyon(), Winding::Positive);

    let mut buffers = VertexBuffers::<Vertex, u32>::new();

    {
        let fill_opts = FillOptions::default();
        let mut tess = FillTessellator::new();
        let mut builder = BuffersBuilder::new(&mut buffers, as_vertex_fn(rect.color));

        _ = tess.tessellate_path(&fill_path.build(), &fill_opts, &mut builder);
    }

    let mut buffers2 = VertexBuffers::<Vertex, u32>::new();

    // ---- STROKE ----
    if rect.stroke_width > 0.0 {
        let mut tess = StrokeTessellator::new();

        // Build stroke options
        let stroke_opts = StrokeOptions::default()
            .with_tolerance(0.01)
            .with_line_width(rect.stroke_width)
            .with_line_join(lyon::tessellation::LineJoin::Round)
            .with_line_cap(lyon::tessellation::LineCap::Round);

        let mut builder =
            BuffersBuilder::new(&mut buffers2, as_stroke_vertex_fn(rect.stroke_color));

        _ = tess.tessellate_path(&stroke_path.build(), &stroke_opts, &mut builder);
    }

    buffers.indices.extend(
        buffers2
            .indices
            .iter()
            .map(|x| x + buffers.vertices.len() as u32),
    );
    buffers.vertices.extend(buffers2.vertices);

    Mesh {
        vertices: buffers.vertices,
        indices: buffers.indices,
    }
}

#[inline(always)]
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
#[inline(always)]
fn as_stroke_vertex_fn(color: BasicColor) -> impl Fn(StrokeVertex<'_, '_>) -> Vertex {
    move |vertex: StrokeVertex<'_, '_>| {
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

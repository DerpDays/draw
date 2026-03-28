use color::LinearSrgb;
use euclid::default::{Box2D, Point2D, SideOffsets2D};
use graphics::{BasicColor, make_positive_box, primitives::Rectangle};
use lyon::{
    path::{Path, Winding},
    tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers},
};

use crate::{
    Mesh,
    shaders::generic::{Vertex, VertexKind},
};

#[profiling::function]
pub fn render_rectangle(rect: &Rectangle) -> Mesh<Vertex> {
    if !rect.rounding.is_zero() {
        return render_rounded_rectangle(rect);
    }

    let area = make_positive_box(
        Box2D::from_origin_and_size(rect.origin, rect.size).inner_box(rect.border),
    );
    let rect_count = 1
        + (rect.border.top != 0.) as usize
        + (rect.border.bottom != 0.) as usize
        + (rect.border.left != 0.) as usize
        + (rect.border.right != 0.) as usize;

    let vertices = Vec::with_capacity(4 * rect_count);
    let indices = Vec::with_capacity(6 * rect_count);
    let mut mesh = Mesh { vertices, indices };

    // base rect
    basic_quad(&mut mesh, area, &rect.color);

    if rect.border.top != 0.0 {
        let top_area = make_positive_box(Box2D::new(
            Point2D::new(area.min.x - rect.border.left, area.min.y - rect.border.top),
            Point2D::new(area.max.x + rect.border.right, area.min.y),
        ));
        basic_quad(&mut mesh, top_area, &rect.border_color);
    }
    if rect.border.bottom != 0.0 {
        let bottom_area = make_positive_box(Box2D::new(
            Point2D::new(area.min.x - rect.border.left, area.max.y),
            Point2D::new(
                area.max.x + rect.border.right,
                area.max.y + rect.border.bottom,
            ),
        ));
        basic_quad(&mut mesh, bottom_area, &rect.border_color);
    }
    if rect.border.left != 0.0 {
        let left_area = make_positive_box(Box2D::new(
            Point2D::new(area.min.x - rect.border.left, area.min.y),
            Point2D::new(area.min.x, area.max.y),
        ));
        basic_quad(&mut mesh, left_area, &rect.border_color);
    }

    if rect.border.right != 0.0 {
        let right_area = make_positive_box(Box2D::new(
            Point2D::new(area.max.x, area.min.y),
            Point2D::new(area.max.x + rect.border.right, area.max.y),
        ));
        basic_quad(&mut mesh, right_area, &rect.border_color);
    }

    mesh
}

#[inline(always)]
pub(crate) fn basic_quad(mesh: &mut Mesh<Vertex>, area: Box2D<f32>, color: &BasicColor) {
    let start_idx = mesh.vertices.len() as u32;
    mesh.vertices.extend_from_slice(&match color {
        BasicColor::Solid(color) => Vertex::new_solid_rect(
            area.min.to_array(),
            area.max.to_array(),
            color.convert().premultiply(),
        ),
        BasicColor::LinearGradient(gradient) => {
            Vertex::new_gradient_rect(area.min.to_array(), area.max.to_array(), gradient)
        }
    });
    mesh.indices.extend_from_slice(&[
        start_idx,
        start_idx + 1,
        start_idx + 2,
        start_idx,
        start_idx + 2,
        start_idx + 3,
    ]);
}

#[inline(always)]
#[profiling::function]
fn render_rounded_rectangle(rect: &Rectangle) -> Mesh<Vertex> {
    let outer_area = make_positive_box(Box2D::from_origin_and_size(rect.origin, rect.size));
    let inner_area = outer_area.inner_box(rect.border);

    let inner_radii = rect.rounding.to_lyon();
    let outer_radii = add_border_to_radii(&inner_radii, &rect.border);

    let mut buffers = VertexBuffers::<Vertex, u32>::new();
    let mut tess = FillTessellator::new();
    let fill_opts = FillOptions::default().with_tolerance(0.01);

    if !inner_area.is_empty() {
        let mut fill_builder = Path::builder();
        fill_builder.add_rounded_rectangle(&inner_area, &inner_radii, Winding::Positive);
        let fill_path = fill_builder.build();

        let mut vertex_builder = BuffersBuilder::new(&mut buffers, as_vertex_fn(rect.color));

        _ = tess.tessellate_path(&fill_path, &fill_opts, &mut vertex_builder);
    }

    if !rect.border.is_zero() {
        let mut border_builder = Path::builder();
        border_builder.add_rounded_rectangle(&outer_area, &outer_radii, Winding::Positive);

        // Subtract inner using negative winding
        if !inner_area.is_empty() {
            border_builder.add_rounded_rectangle(&inner_area, &inner_radii, Winding::Negative);
        }

        let border_path = border_builder.build();

        let mut vertex_builder = BuffersBuilder::new(&mut buffers, as_vertex_fn(rect.border_color));

        _ = tess.tessellate_path(&border_path, &fill_opts, &mut vertex_builder);
    }
    Mesh {
        vertices: buffers.vertices,
        indices: buffers.indices,
    }
}

/// Helper to calculate OUTER radii based on inner radii + border width.
fn add_border_to_radii(
    inner_radii: &lyon::tessellation::path::builder::BorderRadii,
    border: &SideOffsets2D<f32>,
) -> lyon::tessellation::path::builder::BorderRadii {
    // Helper to add border width to radius.
    // We use the MAX of the two adjacent borders.
    // If we used the average or min, the outer corner might be too "sharp"
    // and clip into the border width visually.
    let add = |r: f32, b_adj: f32, b_opp: f32| -> f32 { r + b_adj.max(b_opp) };

    lyon::tessellation::path::builder::BorderRadii {
        top_left: add(inner_radii.top_left, border.top, border.left),
        top_right: add(inner_radii.top_right, border.top, border.right),
        bottom_left: add(inner_radii.bottom_left, border.bottom, border.left),
        bottom_right: add(inner_radii.bottom_right, border.bottom, border.right),
    }
}

#[inline(always)]
fn as_vertex_fn(color: BasicColor) -> impl Fn(FillVertex<'_>) -> Vertex {
    move |vertex: FillVertex<'_>| Vertex {
        position: vertex.position().to_array(),
        color: match color {
            BasicColor::Solid(color) => color.convert::<LinearSrgb>().premultiply().components,
            BasicColor::LinearGradient(gradient) => {
                gradient
                    .get_point(vertex.position())
                    .convert::<LinearSrgb>()
                    .premultiply()
                    .components
            }
        },
        kind: VertexKind::Color as u32,
        texture: 0,
        tex_coords: [0., 0.],
    }
}

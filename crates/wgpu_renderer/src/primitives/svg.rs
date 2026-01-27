use color::AlphaColor;
use euclid::default::{Point2D, Transform2D, Vector2D};
use graphics::primitives::Svg;
use lyon::{
    path::{FillRule, LineCap, LineJoin},
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
use usvg::{Node, Paint, PaintOrder, Tree, tiny_skia_path::PathSegment};

use crate::{GraphicsContext, Mesh, PrimitiveCache, shaders::generic::Vertex};

pub fn render_svg(
    _: &mut GraphicsContext,
    svg: &Svg,
    _: &mut Option<PrimitiveCache>,
) -> Mesh<Vertex> {
    let mut result = Mesh::empty();
    let Ok(tree) = Tree::from_data(&svg.data, &usvg::Options::default()) else {
        log::error!("tried to render an invalid SVG");
        return Mesh::empty();
    };

    // Push root's children in reverse order onto the stack
    let mut stack: Vec<&Node> = tree.root().children().iter().rev().collect();

    let root_transformation = Transform2D::scale(
        svg.size.width / tree.size().width(),
        svg.size.height / tree.size().height(),
    )
    .then_translate(Vector2D::new(svg.origin.x, svg.origin.y));

    let quantize = false;

    while let Some(node) = stack.pop() {
        match node {
            Node::Group(group) => {
                for child in group.children().iter().rev() {
                    stack.push(child);
                }
            }
            Node::Path(path) => {
                let transform = path.abs_transform();

                let transform = Transform2D::new(
                    transform.sx,
                    transform.kx,
                    transform.ky,
                    transform.sy,
                    transform.tx,
                    transform.ty,
                );
                let mut builder = lyon::path::Path::svg_builder()
                    .transformed(transform.then(&root_transformation));
                for segment in path.data().segments() {
                    match segment {
                        PathSegment::MoveTo(point) => {
                            builder.move_to(Point2D::new(point.x, point.y));
                        }
                        PathSegment::LineTo(point) => {
                            builder.line_to(Point2D::new(point.x, point.y));
                        }
                        PathSegment::QuadTo(point, point1) => {
                            builder.quadratic_bezier_to(
                                Point2D::new(point.x, point.y),
                                Point2D::new(point1.x, point1.y),
                            );
                        }
                        PathSegment::CubicTo(point, point1, point2) => {
                            builder.cubic_bezier_to(
                                Point2D::new(point.x, point.y),
                                Point2D::new(point1.x, point1.y),
                                Point2D::new(point2.x, point2.y),
                            );
                        }
                        PathSegment::Close => {
                            builder.close();
                        }
                    }
                }
                let lyon_path = builder.build();
                match path.paint_order() {
                    PaintOrder::FillAndStroke => {
                        if let Some(fill) = path.fill() {
                            result.append_mesh(fill_path(&lyon_path, fill, quantize))
                        }
                        if let Some(stroke) = path.stroke() {
                            result.append_mesh(stroke_path(&lyon_path, stroke, quantize))
                        }
                    }
                    PaintOrder::StrokeAndFill => {
                        if let Some(stroke) = path.stroke() {
                            result.append_mesh(stroke_path(&lyon_path, stroke, quantize))
                        }
                        if let Some(fill) = path.fill() {
                            result.append_mesh(fill_path(&lyon_path, fill, quantize))
                        }
                    }
                }
            }
            Node::Image(_image) => {
                // TODO: support images
                log::warn!("image element in svg not yet implemented!");
            }
            Node::Text(text) => {
                let _transform = text.abs_transform();
                for span in text.layouted() {
                    for glyph in &span.positioned_glyphs {
                        // apply abs transform
                        let _transform = glyph.transform();
                    }
                }

                // TODO: support text
                log::warn!("text element in svg not yet implemented!");
            }
        }
    }
    result
}

pub fn fill_path(lyon_path: &lyon::path::Path, fill: &usvg::Fill, quantize: bool) -> Mesh<Vertex> {
    let color = match fill.paint() {
        Paint::Color(paint_color) => AlphaColor::from_rgba8(
            paint_color.red,
            paint_color.green,
            paint_color.blue,
            fill.opacity().to_u8(),
        ),
        // TODO: support fill types for svg
        _ => {
            log::error!("fill types other than color are not supported for svg yet");
            AlphaColor::TRANSPARENT
        }
    };
    let fill_rule = match fill.rule() {
        usvg::FillRule::NonZero => FillRule::NonZero,
        usvg::FillRule::EvenOdd => FillRule::EvenOdd,
    };

    let mut tessellator = FillTessellator::new();
    let mut buffers = VertexBuffers::<Vertex, u32>::new();
    let mut builder = BuffersBuilder::new(&mut buffers, |vertex: FillVertex<'_>| {
        Vertex::new_color(
            if quantize {
                vertex.position().round().to_array()
            } else {
                vertex.position().to_array()
            },
            color.convert().premultiply(),
        )
    });
    // TODO: Properly fill with the right colors, etc.
    let options = FillOptions::default()
        .with_tolerance(0.01)
        .with_fill_rule(fill_rule);
    _ = tessellator.tessellate_path(lyon_path, &options, &mut builder);

    Mesh {
        vertices: buffers.vertices,
        indices: buffers.indices,
    }
}

pub fn stroke_path(
    lyon_path: &lyon::path::Path,
    stroke: &usvg::Stroke,
    quantize: bool,
) -> Mesh<Vertex> {
    let color = match stroke.paint() {
        Paint::Color(paint_color) => AlphaColor::from_rgba8(
            paint_color.red,
            paint_color.green,
            paint_color.blue,
            stroke.opacity().to_u8(),
        ),
        // TODO: support fill types for svg
        _ => {
            log::error!("fill types other than color are not supported for svg yet");
            AlphaColor::TRANSPARENT
        }
    };
    let stroke_width = stroke.width().get();
    let line_cap = match stroke.linecap() {
        usvg::LineCap::Butt => LineCap::Butt,
        usvg::LineCap::Round => LineCap::Round,
        usvg::LineCap::Square => LineCap::Square,
    };
    let miter_limit = stroke.miterlimit().get();
    let line_join = match stroke.linejoin() {
        usvg::LineJoin::Miter => LineJoin::Miter,
        usvg::LineJoin::MiterClip => LineJoin::MiterClip,
        usvg::LineJoin::Round => LineJoin::Round,
        usvg::LineJoin::Bevel => LineJoin::Bevel,
    };

    let mut tessellator = StrokeTessellator::new();
    let mut buffers = VertexBuffers::<Vertex, u32>::new();
    let mut builder = BuffersBuilder::new(&mut buffers, |vertex: StrokeVertex<'_, '_>| {
        Vertex::new_color(
            if quantize {
                vertex.position().round().to_array()
            } else {
                vertex.position().to_array()
            },
            color.convert().premultiply(),
        )
    });
    // TODO: Properly fill with the right colors, etc.
    let options = StrokeOptions::default()
        .with_tolerance(0.01)
        .with_line_width(stroke_width)
        .with_start_cap(line_cap)
        .with_end_cap(line_cap)
        .with_miter_limit(miter_limit)
        .with_line_join(line_join);
    _ = tessellator.tessellate_path(lyon_path, &options, &mut builder);

    Mesh {
        vertices: buffers.vertices,
        indices: buffers.indices,
    }
}

use std::marker::PhantomData;

use color::{PremulColor, Srgb};
use euclid::default::{Box2D, Point2D, Size2D, Transform2D, Vector2D};
use lyon::path::{FillRule, LineCap, LineJoin};
use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, StrokeOptions, StrokeTessellator,
    StrokeVertex, VertexBuffers,
};
use serde::{Deserialize, Serialize};
use usvg::tiny_skia_path::PathSegment;
use usvg::{Node, Paint, PaintOrder, Tree};

use crate::{ApplyCoordinates, Drawable, Mesh, Systems};
use crate::{Vertex, VertexKind};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Options {
    pub fill_color: Option<PremulColor<Srgb>>,
    pub stroke_color: Option<PremulColor<Srgb>>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            fill_color: None,
            stroke_color: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Svg<C: ApplyCoordinates> {
    #[serde(skip)]
    render_cache: Option<Mesh<Vertex>>,

    origin: Point2D<f32>,
    size: Size2D<f32>,
    data: Vec<u8>,

    options: Options,
    _marker: PhantomData<C>,
}

impl<C: ApplyCoordinates> Svg<C> {
    /// Creates a new SVG primitive, where the origin corresponds to the top-left corner.
    pub fn new(origin: Point2D<f32>, size: Size2D<f32>, data: Vec<u8>, options: Options) -> Self {
        Self {
            render_cache: None,

            origin,
            size,
            data,

            options,
            _marker: PhantomData,
        }
    }

    pub fn clear_cache(&mut self) {
        self.render_cache = None;
    }

    pub fn update_rect(&mut self, origin: Point2D<f32>, size: Size2D<f32>) {
        self.origin = origin;
        self.size = size;
        self.clear_cache();
    }

    /// Translate the svg by a given amount, applying the transformation to the cache without
    /// re-tesselating the svg.
    pub fn translate(&mut self, dx: Vector2D<f32>) {
        self.origin = self.origin + dx;
        if let Some(cache) = &mut self.render_cache {
            cache.translate(dx);
        }
    }

    pub fn update_options(&mut self, options: Options) {
        self.options = options;
        self.clear_cache();
    }

    pub fn fill_path(&self, lyon_path: &lyon::path::Path, fill: &usvg::Fill) -> Mesh<Vertex> {
        let color = self.options.fill_color.unwrap_or(match fill.paint() {
            Paint::Color(paint_color) => PremulColor::from_rgba8(
                paint_color.red,
                paint_color.green,
                paint_color.blue,
                fill.opacity().to_u8(),
            ),
            // TODO: support fill types for svg
            _ => {
                tracing::error!("fill types other than color are not supported for svg yet");
                PremulColor::TRANSPARENT
            }
        });
        let fill_rule = match fill.rule() {
            usvg::FillRule::NonZero => FillRule::NonZero,
            usvg::FillRule::EvenOdd => FillRule::EvenOdd,
        };

        let mut tessellator = FillTessellator::new();
        let mut buffers = VertexBuffers::<Vertex, u32>::new();
        let mut builder = BuffersBuilder::new(&mut buffers, |vertex: FillVertex<'_>| {
            Vertex::with_color(vertex.position(), C::apply(VertexKind::Color(color)))
        });
        // TODO: Properly fill with the right colors, etc.
        let options = FillOptions::default()
            .with_tolerance(0.1)
            .with_fill_rule(fill_rule);
        _ = tessellator.tessellate_path(lyon_path, &options, &mut builder);

        Mesh {
            vertices: buffers.vertices,
            indices: buffers.indices,
        }
    }

    pub fn stroke_path(&self, lyon_path: &lyon::path::Path, stroke: &usvg::Stroke) -> Mesh<Vertex> {
        let color = self.options.stroke_color.unwrap_or(match stroke.paint() {
            Paint::Color(paint_color) => PremulColor::from_rgba8(
                paint_color.red,
                paint_color.green,
                paint_color.blue,
                stroke.opacity().to_u8(),
            ),
            // TODO: support fill types for svg
            _ => {
                tracing::error!("fill types other than color are not supported for svg yet");
                PremulColor::TRANSPARENT
            }
        });
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
            Vertex::with_color(vertex.position(), C::apply(VertexKind::Color(color)))
        });
        // TODO: Properly fill with the right colors, etc.
        let options = StrokeOptions::default()
            .with_tolerance(0.1)
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
}

impl<C: ApplyCoordinates> Drawable for Svg<C> {
    fn render(&mut self, _: &mut Systems) -> &Mesh<Vertex> {
        if let Some(ref cache) = self.render_cache {
            return cache;
        }

        let mut result = Mesh::empty();
        let tree = Tree::from_data(self.data.as_slice(), &usvg::Options::default())
            .expect("invalid svg --- TODO: dont die");

        // Push root's children in reverse order onto the stack
        let mut stack: Vec<&Node> = tree.root().children().iter().rev().collect();

        let root_transformation = Transform2D::scale(
            self.size.width / tree.size().width(),
            self.size.height / tree.size().height(),
        )
        .then_translate(Vector2D::new(self.origin.x, self.origin.y));

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
                                result.append(&self.fill_path(&lyon_path, fill))
                            }
                            if let Some(stroke) = path.stroke() {
                                result.append(&self.stroke_path(&lyon_path, stroke))
                            }
                        }
                        PaintOrder::StrokeAndFill => {
                            if let Some(stroke) = path.stroke() {
                                result.append(&self.stroke_path(&lyon_path, stroke))
                            }
                            if let Some(fill) = path.fill() {
                                result.append(&self.fill_path(&lyon_path, fill))
                            }
                        }
                    }
                }
                Node::Image(_image) => {
                    // TODO: support images
                    tracing::warn!("image element in svg not yet implemented!");
                }
                Node::Text(text) => {
                    let transform = text.abs_transform();
                    for span in text.layouted() {
                        for glyph in &span.positioned_glyphs {
                            // apply abs transform
                            let transform = glyph.transform();
                        }
                    }

                    // TODO: support text
                    tracing::warn!("text element in svg not yet implemented!");
                }
            }
        }

        self.render_cache = Some(result);
        self.render_cache.as_ref().unwrap()
    }
    fn bounding_box(&self) -> Box2D<f32> {
        let mut area = Box2D::from_origin_and_size(self.origin, self.size);
        // We can receive negative areas in Box2D since size can also be negative,
        // therefore we need to change ensure each axis are their actual minimum/maximum.
        if area.min.x > area.max.x {
            std::mem::swap(&mut area.min.x, &mut area.max.x);
        }
        if area.min.y > area.max.y {
            std::mem::swap(&mut area.min.y, &mut area.max.y);
        }
        area
    }

    fn is_dirty(&self) -> bool {
        self.render_cache.is_none()
    }
}

use std::marker::PhantomData;

use color::{PremulColor, Srgb};
use euclid::default::{Box2D, Point2D, Size2D};
use lyon::path::{Path, Winding};
use lyon::tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers};
use serde::{Deserialize, Serialize};

use crate::{make_positive_box, BasicColor, BasicLinearGradient, Vertex};
use crate::{ApplyCoordinates, Drawable, Mesh, Systems, VertexKind};
use crate::{BoxSizing, Rounding};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Options {
    pub color: BasicColor,
    pub stroke_color: BasicColor,
    pub stroke_width: f32,
    pub rounding: Rounding,
    pub box_sizing: BoxSizing,
}
impl Options {
    pub const DEFAULT: Self = Self {
        color: BasicColor::Solid(PremulColor::new([0.4, 0.4, 0.4, 1.])),
        stroke_color: BasicColor::Solid(PremulColor::new([1., 1., 1., 1.])),
        stroke_width: 0.,
        rounding: Rounding::DEFAULT,
        box_sizing: BoxSizing::BorderBox,
    };
    pub fn only_color(color: impl Into<BasicColor>) -> Self {
        Self {
            color: color.into(),
            ..Self::DEFAULT
        }
    }
}

impl Default for Options {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rectangle<C: ApplyCoordinates> {
    fill_path: Path,
    stroke_path: Path,

    #[serde(skip)]
    render_cache: Option<Mesh<Vertex>>,

    origin: Point2D<f32>,
    size: Size2D<f32>,

    options: Options,
    _marker: PhantomData<C>,
}

impl<C: ApplyCoordinates> Rectangle<C> {
    pub fn new(origin: Point2D<f32>, size: Size2D<f32>, options: Options) -> Self {
        let (fill_path, stroke_path) = Self::build_path(&origin, &size, &options);
        let mut res = Self {
            fill_path,
            stroke_path,
            render_cache: None,

            origin,
            size,

            options,
            _marker: PhantomData,
        };
        res.apply_area_to_color();
        res
    }
    fn build_path(origin: &Point2D<f32>, size: &Size2D<f32>, options: &Options) -> (Path, Path) {
        let mut fill_path = Path::builder();
        let mut stroke_path = Path::builder();

        // We can receive negative areas in Box2D since size can also be negative,
        // therefore we need to change ensure each axis are their actual minimum/maximum.
        let area = make_positive_box(Box2D::from_origin_and_size(*origin, *size));

        if options.stroke_width == 0. {
            fill_path.add_rounded_rectangle(&area, &options.rounding.to_lyon(), Winding::Positive);
        } else {
            let (inner, outer) = match options.box_sizing {
                BoxSizing::BorderBox => (
                    area.inner_box(euclid::SideOffsets2D::new_all_same(options.stroke_width)),
                    area,
                ),
                BoxSizing::ContentBox => (
                    area,
                    make_positive_box(
                        area.outer_box(euclid::SideOffsets2D::new_all_same(options.stroke_width)),
                    ),
                ),
            };
            if !inner.is_empty() {
                fill_path.add_rounded_rectangle(
                    &inner,
                    &options.rounding.to_lyon(),
                    Winding::Positive,
                );
            }
            stroke_path.add_rounded_rectangle(
                &outer,
                &options.rounding.to_lyon(),
                Winding::Positive,
            );
        }
        (fill_path.build(), stroke_path.build())
    }
    fn apply_area_to_color(&mut self) {
        let bounding = self.bounding_box();
        match self.options.color {
            BasicColor::Solid(_) => {}
            BasicColor::LinearGradient(ref mut gradient) => {
                gradient.update_points(bounding.min, Point2D::new(bounding.max.x, bounding.min.y));
            }
        }
    }
    pub fn update_area(&mut self, origin: Point2D<f32>, size: Size2D<f32>) {
        self.origin = origin;
        self.size = size;
        self.apply_area_to_color();
        (self.fill_path, self.stroke_path) = Self::build_path(&origin, &size, &self.options);
        self.clear_cache();
    }
    pub fn update_options(&mut self, options: Options) {
        self.options = options;
        self.apply_area_to_color();
        self.clear_cache();
    }

    #[inline]
    pub fn options(&self) -> &Options {
        &self.options
    }

    #[inline]
    pub fn clear_cache(&mut self) {
        self.render_cache = None;
    }

    // TODO: rename
    pub fn resize_to_point(&mut self, position: lyon::math::Point) {
        self.set_size(Size2D::from(position - self.origin));
    }
    pub fn resize_to_point_square(&mut self, position: lyon::math::Point) {
        let delta = position - self.origin;
        let length = delta.x.abs().min(delta.y.abs());
        let width = length * delta.x.signum();
        let height = length * delta.y.signum();

        self.set_size(Size2D::new(width, height));
    }

    #[inline]
    pub const fn size(&self) -> Size2D<f32> {
        self.size
    }
    #[inline]
    pub const fn origin(&self) -> Point2D<f32> {
        self.origin
    }

    pub fn set_size(&mut self, size: Size2D<f32>) {
        self.size = size;
        self.apply_area_to_color();
        (self.fill_path, self.stroke_path) = Self::build_path(&self.origin, &size, &self.options);
        self.clear_cache();
    }

    fn vertex_linear_gradient(gradient: &BasicLinearGradient, vertex: &FillVertex<'_>) -> Vertex {
        Vertex::with_color(
            vertex.position(),
            C::apply(VertexKind::Color(gradient.get_point(vertex.position()))),
        )
    }
    fn vertex_solid(color: PremulColor<Srgb>, vertex: &FillVertex<'_>) -> Vertex {
        Vertex::with_color(vertex.position(), C::apply(VertexKind::Color(color)))
    }
    fn as_vertex_fn(color: BasicColor) -> impl Fn(FillVertex<'_>) -> Vertex {
        move |vertex: FillVertex<'_>| match color {
            BasicColor::Solid(color) => Self::vertex_solid(color, &vertex),
            BasicColor::LinearGradient(gradient) => {
                Self::vertex_linear_gradient(&gradient, &vertex)
            }
        }
    }
}

impl<C: ApplyCoordinates> Drawable for Rectangle<C> {
    fn render(&mut self, _: &mut Systems) -> &Mesh<Vertex> {
        if let Some(ref cache) = self.render_cache {
            return cache;
        }

        let mut buffers = VertexBuffers::<Vertex, u32>::new();
        let options = FillOptions::default();
        let mut tessellator = FillTessellator::new();

        let mut stroke_builder =
            BuffersBuilder::new(&mut buffers, Self::as_vertex_fn(self.options.stroke_color));
        _ = tessellator.tessellate_path(&self.stroke_path, &options, &mut stroke_builder);

        let mut fill_builder =
            BuffersBuilder::new(&mut buffers, Self::as_vertex_fn(self.options.color));
        _ = tessellator.tessellate_path(&self.fill_path, &options, &mut fill_builder);

        self.render_cache = Some(Mesh {
            vertices: buffers.vertices.clone(),
            indices: buffers.indices.clone(),
        });
        self.render_cache.as_ref().unwrap()
    }
    fn bounding_box(&self) -> lyon::geom::Box2D<f32> {
        let area = Box2D::from_origin_and_size(self.origin, self.size);
        // We can receive negative areas in Box2D since size can also be negative,
        // therefore we need to change ensure each axis are their actual minimum/maximum.
        make_positive_box(area)
    }

    fn is_dirty(&self) -> bool {
        self.render_cache.is_none()
    }
}

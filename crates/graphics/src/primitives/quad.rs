use std::marker::PhantomData;

use color::{PremulColor, Srgb};
use euclid::default::{Box2D, Point2D};
use serde::{Deserialize, Serialize};

use crate::{make_positive_box, Vertex, VertexKind};
use crate::{ApplyCoordinates, Drawable, Mesh, Systems};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Options {
    pub p1: PremulColor<Srgb>,
    pub p2: PremulColor<Srgb>,
    pub p3: PremulColor<Srgb>,
    pub p4: PremulColor<Srgb>,
}
impl Options {
    pub const DEFAULT: Self = Self {
        p1: PremulColor::new([0.4, 0.4, 0.4, 1.]),
        p2: PremulColor::new([0.4, 0.4, 0.4, 1.]),
        p3: PremulColor::new([0.4, 0.4, 0.4, 1.]),
        p4: PremulColor::new([0.4, 0.4, 0.4, 1.]),
    };
    pub const fn solid(color: PremulColor<Srgb>) -> Self {
        Self {
            p1: color,
            p2: color,
            p3: color,
            p4: color,
        }
    }
    pub const fn linear_gradient(start: PremulColor<Srgb>, end: PremulColor<Srgb>) -> Self {
        Self {
            p1: start,
            p2: end,
            p3: start,
            p4: end,
        }
    }
}

impl Default for Options {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Order of points drawn goes.
/// 1 -- 2
/// |  /
/// | /
/// 3 -- 4
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct QuadPoints {
    pub p1: Point2D<f32>,
    pub p2: Point2D<f32>,
    pub p3: Point2D<f32>,
    pub p4: Point2D<f32>,
}

impl QuadPoints {
    /// A 2D axis aligned box created from min/max points.
    pub const fn zero() -> Self {
        Self {
            p1: Point2D::new(0., 0.),
            p2: Point2D::new(0., 0.),
            p3: Point2D::new(0., 0.),
            p4: Point2D::new(0., 0.),
        }
    }
    pub const fn box2d(min: Point2D<f32>, max: Point2D<f32>) -> Self {
        debug_assert!(
            min.x <= max.x && min.y <= max.y,
            "attempted to create a quad from box2d(..) with invalid min/max points."
        );
        Self {
            p1: min,
            p2: Point2D::new(max.x, min.y),
            p3: Point2D::new(min.x, max.y),
            p4: max,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Quad<C: ApplyCoordinates> {
    #[serde(skip)]
    render_cache: Option<Mesh<Vertex>>,

    points: QuadPoints,

    options: Options,
    _marker: PhantomData<C>,
}

impl<C: ApplyCoordinates> Quad<C> {
    pub const fn new(points: QuadPoints, options: Options) -> Self {
        Self {
            render_cache: None,

            points,

            options,
            _marker: PhantomData,
        }
    }
    pub fn update_options(&mut self, options: Options) {
        self.options = options;
        self.clear_cache();
    }
    pub fn update_points(&mut self, points: QuadPoints) {
        self.points = points;
        self.clear_cache();
    }
    pub fn options(&self) -> &Options {
        &self.options
    }

    #[inline]
    pub fn clear_cache(&mut self) {
        self.render_cache = None;
    }
}

impl<C: ApplyCoordinates> Drawable for Quad<C> {
    fn render(&mut self, _: &mut Systems) -> &Mesh<Vertex> {
        if let Some(ref cache) = self.render_cache {
            return cache;
        }

        self.render_cache = Some(Mesh {
            vertices: vec![
                Vertex::with_color(self.points.p1, C::apply(VertexKind::Color(self.options.p1))),
                Vertex::with_color(self.points.p2, C::apply(VertexKind::Color(self.options.p2))),
                Vertex::with_color(self.points.p4, C::apply(VertexKind::Color(self.options.p4))),
                Vertex::with_color(self.points.p3, C::apply(VertexKind::Color(self.options.p3))),
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
        });
        self.render_cache.as_ref().unwrap()
    }
    fn bounding_box(&self) -> lyon::geom::Box2D<f32> {
        let area = Box2D::from_points([
            self.points.p1,
            self.points.p2,
            self.points.p3,
            self.points.p4,
        ]);
        // We can receive negative areas in Box2D since size can also be negative,
        // therefore we need to change ensure each axis are their actual minimum/maximum.
        make_positive_box(area)
    }

    fn is_dirty(&self) -> bool {
        self.render_cache.is_none()
    }
}

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
}
impl Options {
    pub const DEFAULT: Self = Self {
        p1: PremulColor::new([0.4, 0.4, 0.4, 1.]),
        p2: PremulColor::new([0.4, 0.4, 0.4, 1.]),
        p3: PremulColor::new([0.4, 0.4, 0.4, 1.]),
    };
}

impl Default for Options {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Triangle<C: ApplyCoordinates> {
    #[serde(skip)]
    render_cache: Option<Mesh<Vertex>>,

    p1: Point2D<f32>,
    p2: Point2D<f32>,
    p3: Point2D<f32>,

    options: Options,
    _marker: PhantomData<C>,
}

impl<C: ApplyCoordinates> Triangle<C> {
    pub fn new(p1: Point2D<f32>, p2: Point2D<f32>, p3: Point2D<f32>, options: Options) -> Self {
        Self {
            render_cache: None,

            p1,
            p2,
            p3,

            options,
            _marker: PhantomData,
        }
    }
    pub fn update_options(&mut self, options: Options) {
        self.options = options;
        self.render_cache = None;
    }
    pub fn update_points(&mut self, p1: Point2D<f32>, p2: Point2D<f32>, p3: Point2D<f32>) {
        self.p1 = p1;
        self.p2 = p2;
        self.p3 = p3;
        self.render_cache = None;
    }
    pub fn options(&self) -> &Options {
        &self.options
    }
}

impl<C: ApplyCoordinates> Drawable for Triangle<C> {
    fn render(&mut self, _: &mut Systems) -> &Mesh<Vertex> {
        if let Some(ref cache) = self.render_cache {
            return cache;
        }

        self.render_cache = Some(Mesh {
            vertices: vec![
                Vertex::with_color(self.p1, C::apply(VertexKind::Color(self.options.p1))),
                Vertex::with_color(self.p2, C::apply(VertexKind::Color(self.options.p2))),
                Vertex::with_color(self.p3, C::apply(VertexKind::Color(self.options.p3))),
            ],
            indices: vec![0, 1, 2],
        });
        self.render_cache.as_ref().unwrap()
    }
    fn bounding_box(&self) -> lyon::geom::Box2D<f32> {
        let area = Box2D::from_points([self.p1, self.p2, self.p3]);
        // We can receive negative areas in Box2D since size can also be negative,
        // therefore we need to change ensure each axis are their actual minimum/maximum.
        make_positive_box(area)
    }

    fn is_dirty(&self) -> bool {
        self.render_cache.is_none()
    }
}

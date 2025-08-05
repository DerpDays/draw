use std::marker::PhantomData;

use color::{PremulColor, Srgb};
use euclid::default::Box2D;
use lyon::math::{Angle, Point, Vector};
use lyon::path::{Path, Winding};
use lyon::tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::{ApplyCoordinates, Drawable, Mesh, Systems, Vertex, VertexKind};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Options {
    pub color: PremulColor<Srgb>,
    pub stroke_color: PremulColor<Srgb>,
    pub stroke_width: f32,
    pub text: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            color: PremulColor::new([1., 0., 0., 0.5]),
            stroke_color: PremulColor::new([0., 1., 0., 0.5]),
            stroke_width: 1.,
            text: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ellipse<C: ApplyCoordinates> {
    path: Path,

    #[serde(skip)]
    render_cache: Option<Mesh<Vertex>>,

    origin: Point,
    radius: Vector,

    options: Options,
    _marker: PhantomData<C>,
}

impl<C: ApplyCoordinates> Ellipse<C> {
    pub fn new(origin: Point, radius: Vector, options: Options) -> Self {
        Self {
            path: Self::build_path(&origin, radius),
            render_cache: None,

            origin,
            radius,

            options,
            _marker: PhantomData,
        }
    }
    fn build_path(origin: &Point, radius: Vector) -> Path {
        let mut builder = Path::builder();
        builder.add_ellipse(*origin, radius, Angle::zero(), Winding::Positive);
        builder.build()
    }
    pub fn resize_to_point(&mut self, position: lyon::math::Point) {
        // rebuild path
        self.radius = (position - self.origin).abs();
        self.path = Self::build_path(&self.origin, self.radius);
        // clear tessellation cache
        self.render_cache = None;
    }
    pub fn resize_to_point_square(&mut self, position: lyon::math::Point) {
        // rebuild path
        let radius = (position - self.origin).abs();
        self.radius = radius.max(radius.yx());
        self.path = Self::build_path(&self.origin, self.radius);
        // clear tessellation cache
        self.render_cache = None;
    }

    pub fn resize_square(&mut self) {
        // rebuild path
        self.radius = self.radius.max(self.radius.yx());
        self.path = Self::build_path(&self.origin, self.radius);
        // clear tessellation cache
        self.render_cache = None;
    }

    pub fn radius(&self) -> Vector {
        self.radius
    }
}

impl<C: ApplyCoordinates> Drawable for Ellipse<C> {
    fn render(&mut self, _systems: &mut Systems) -> &Mesh<Vertex> {
        if let Some(ref cache) = self.render_cache {
            return cache;
        };
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
                "Error while tessellating rectangle with options {:?}: {}",
                self.options, err
            );
        }

        tracing::warn!("ellipse vertices: {:?}", buffers.vertices.len());

        self.render_cache = Some(Mesh {
            vertices: buffers.vertices,
            indices: buffers.indices,
        });
        self.render_cache.as_ref().unwrap()
    }

    fn bounding_box(&self) -> Box2D<f32> {
        Box2D::new(self.origin, self.origin).inflate(self.radius.x, self.radius.y)
    }

    fn is_dirty(&self) -> bool {
        self.render_cache.is_none()
    }
}

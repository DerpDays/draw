use std::marker::PhantomData;

use color::{PremulColor, Srgb};
use euclid::default::{Box2D, Point2D, Vector2D};
use lyon::{
    algorithms::aabb::fast_bounding_box,
    geom::LineSegment,
    path::Path,
    tessellation::{BuffersBuilder, StrokeOptions, StrokeTessellator, StrokeVertex, VertexBuffers},
};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::{ApplyCoordinates, Drawable, Mesh, Systems, Vertex, VertexKind};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Options {
    pub color: PremulColor<Srgb>,
    pub width: f32,
    pub text: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            color: PremulColor::new([1., 0., 0., 0.5]),
            width: 3.,
            text: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Line<C: ApplyCoordinates> {
    path: Path,
    #[serde(skip)]
    render_cache: Option<Mesh<Vertex>>,

    origin: Point2D<f32>,
    destination: Point2D<f32>,

    options: Options,
    _marker: PhantomData<C>,
}

impl<C: ApplyCoordinates> Line<C> {
    pub fn new(origin: Point2D<f32>, destination: Point2D<f32>, options: Options) -> Self {
        Self {
            path: Self::build_path(&origin, &destination),
            render_cache: None,

            origin,
            destination,

            options,
            _marker: PhantomData,
        }
    }
    fn build_path(origin: &Point2D<f32>, destination: &Point2D<f32>) -> Path {
        let mut builder = Path::builder();
        builder.add_line_segment(&LineSegment {
            from: *origin,
            to: *destination,
        });
        builder.build()
    }
    pub fn set_destination(&mut self, position: Point2D<f32>) {
        // rebuild path
        self.destination = position;
        self.path = Self::build_path(&self.origin, &self.destination);
        // clear tessellation cache
        self.render_cache = None;
    }

    pub fn set_destination_snap(&mut self, position: Point2D<f32>, snap_rad: f32) {
        let delta = position - self.origin;
        let length = delta.length();

        // Compute angle in radians
        let angle = delta.y.atan2(delta.x);

        // Snap angle to nearest snap_degree
        let snapped_angle = (angle / snap_rad).round() * snap_rad;

        // Create new direction vector with same length
        let snapped_delta = Vector2D::new(snapped_angle.cos(), snapped_angle.sin()) * length;

        self.set_destination(self.origin + snapped_delta);
    }
    pub fn is_empty(&mut self) -> bool {
        self.origin == self.destination
    }
}

impl<C: ApplyCoordinates> Drawable for Line<C> {
    fn render(&mut self, _systems: &mut Systems) -> &Mesh<Vertex> {
        if let Some(ref cache) = self.render_cache {
            return cache;
        }
        let mut buffers = VertexBuffers::<Vertex, u32>::new();
        let mut builder = BuffersBuilder::new(&mut buffers, |vertex: StrokeVertex<'_, '_>| {
            Vertex::with_color(
                vertex.position(),
                C::apply(VertexKind::Color(self.options.color)),
            )
        });

        let options = StrokeOptions::default().with_line_width(self.options.width);
        let mut tessellator = StrokeTessellator::new();

        let tessellation_result = tessellator.tessellate_path(&self.path, &options, &mut builder);
        if let Err(err) = tessellation_result {
            warn!(
                "Error while tessellating rectangle with options {:?}: {}",
                self.options, err
            );
        }

        let result = Mesh {
            vertices: buffers.vertices,
            indices: buffers.indices,
        };

        self.render_cache = Some(result.clone());
        self.render_cache.as_ref().unwrap()
    }

    fn bounding_box(&self) -> Box2D<f32> {
        fast_bounding_box(self.path.as_slice())
    }

    fn is_dirty(&self) -> bool {
        self.render_cache.is_none()
    }
}

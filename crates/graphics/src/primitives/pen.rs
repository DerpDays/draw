use std::marker::PhantomData;

use color::{PremulColor, Srgb};
use euclid::default::Box2D;
use lyon::algorithms::aabb::fast_bounding_box;
use lyon::math::Point;
use lyon::path::{LineJoin, Path};
use lyon::tessellation::{
    BuffersBuilder, StrokeOptions, StrokeTessellator, StrokeVertex, VertexBuffers,
};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::{ApplyCoordinates, Drawable, Mesh, Systems, Vertex, VertexKind};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Options {
    pub color: PremulColor<Srgb>,
    pub width: f32,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            color: PremulColor::new([1., 1., 1., 1.0]),
            width: 2.5,
        }
    }
}
// TODO: split paths for long shapes, this would help with caching as tessellating is costly

// TODO: make small circle when only a single point, useful for writing characters with dots e.g. i
// and also for drawing small details

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pen<C: ApplyCoordinates> {
    path: Path,
    #[serde(skip)]
    render_cache: Option<Mesh<Vertex>>,

    points: Vec<Point>,

    options: Options,
    _marker: PhantomData<C>,
}

impl<C: ApplyCoordinates> Pen<C> {
    pub fn new(start: Point, options: Options) -> Self {
        let points = vec![start];
        Self {
            path: Self::build_path(&points),
            render_cache: None,

            points,

            options,
            _marker: PhantomData,
        }
    }
    fn build_path(points: &Vec<Point>) -> Path {
        let mut builder = Path::builder();
        builder.begin(*points.first().unwrap());
        for point in points {
            builder.line_to(*point);
        }
        builder.end(false);
        builder.build()
    }

    pub fn handle_drag(&mut self, position: lyon::math::Point) {
        // rebuild path
        self.points.push(position);
        self.path = Self::build_path(&self.points);
        // clear tessellation cache
        self.render_cache = None;
    }
    pub fn options(&self) -> &Options {
        &self.options
    }
    pub fn update_options(&mut self, options: Options) {
        self.options = options;
        self.render_cache = None;
    }
}

impl<C: ApplyCoordinates> Drawable for Pen<C> {
    fn render(&mut self, _systems: &mut Systems) -> &Mesh<Vertex> {
        if let Some(ref mesh) = self.render_cache {
            return mesh;
        }
        let mut buffers = VertexBuffers::<Vertex, u32>::new();
        let mut builder = BuffersBuilder::new(&mut buffers, |vertex: StrokeVertex<'_, '_>| {
            Vertex::with_color(
                vertex.position(),
                C::apply(VertexKind::Color(self.options.color)),
            )
        });

        let options = StrokeOptions::default()
            .with_line_width(self.options.width)
            .with_line_join(LineJoin::Round);
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

        self.render_cache = Some(result);
        self.render_cache.as_ref().unwrap()
    }

    fn bounding_box(&self) -> Box2D<f32> {
        fast_bounding_box(self.path.as_slice())
    }
    fn is_dirty(&self) -> bool {
        self.render_cache.is_none()
    }
}

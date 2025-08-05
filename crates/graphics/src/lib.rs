#![feature(const_trait_impl)]

use std::{borrow::Borrow, sync::OnceLock};

use atlas::{TextureMesh, TextureVertex};
use bytemuck::{Pod, Zeroable};
use color::{HueDirection, PremulColor, Srgb};
use euclid::default::{Box2D, Point2D, Vector2D};
use lyon::{math::Point, path::builder::BorderRadii};
use serde::{Deserialize, Serialize};

// pub mod line;
pub mod primitives;
pub mod systems;

pub use primitives::Primitive;
pub use systems::Systems;

// TODO: docs
#[derive(Deserialize, Serialize, Default, Clone, Debug)]
pub struct Mesh<V: Clone + Pod + Zeroable> {
    pub vertices: Vec<V>,
    pub indices: Vec<u32>,
}

impl<V> Mesh<V>
where
    V: Clone + Pod + Zeroable,
{
    pub const fn empty() -> Self {
        Self {
            vertices: vec![],
            indices: vec![],
        }
    }
    pub fn offset_indices(&mut self, offset: u32) {
        for index in self.indices.iter_mut() {
            *index += offset
        }
    }

    pub fn append(&mut self, indexed: &Mesh<V>) {
        let offset = self.vertices.len() as u32;
        self.vertices.extend(&indexed.vertices);
        // TODO: move this to offset_indices
        self.indices
            .extend(indexed.indices.iter().map(|i| offset + i))
    }
    pub fn from_slice<T: Borrow<Mesh<V>>>(meshes: &[T]) -> Self {
        let (vertices_len, indices_len) = meshes.iter().fold((0, 0), |prev, mesh| {
            let mesh = mesh.borrow();
            (prev.0 + mesh.vertices.len(), prev.1 + mesh.indices.len())
        });
        let mut result = Mesh {
            vertices: Vec::with_capacity(vertices_len),
            indices: Vec::with_capacity(indices_len),
        };
        for tess in meshes {
            result.append(tess.borrow());
        }
        result
    }
}

pub fn get_empty_mesh() -> &'static Mesh<Vertex> {
    pub static EMPTY_MESH: OnceLock<Mesh<Vertex>> = OnceLock::new();
    EMPTY_MESH.get_or_init(|| Mesh::empty())
}

impl Mesh<Vertex> {
    pub fn new_color_quad(area: lyon::geom::Box2D<f32>, kind: VertexKind) -> Self {
        let vertices = vec![
            Vertex::with_color(area.min, kind),
            Vertex::with_color(Point::new(area.max.x, area.min.y), kind),
            Vertex::with_color(area.max, kind),
            Vertex::with_color(Point::new(area.min.x, area.max.y), kind),
        ];
        let indices = vec![0, 1, 2, 0, 2, 3];

        Mesh { vertices, indices }
    }
    pub fn from_texture_mesh(texture_mesh: TextureMesh, kind: VertexKind) -> Self {
        Self {
            vertices: texture_mesh
                .vertices
                .into_iter()
                .map(|vert| Vertex::from_texture_vertex(vert, kind))
                .collect(),
            indices: texture_mesh.indices,
        }
    }

    pub fn translate(&mut self, dx: euclid::default::Vector2D<f32>) {
        for vertex in self.vertices.iter_mut() {
            vertex.position = [vertex.position[0] + dx.x, vertex.position[1] + dx.y];
        }
    }
}

pub trait Drawable<V = Vertex>
where
    V: Clone + Pod + Zeroable,
{
    fn render(&mut self, systems: &mut Systems) -> &Mesh<V>;
    fn is_dirty(&self) -> bool;
    fn bounding_box(&self) -> Box2D<f32>;
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum VertexKind {
    // Canvas space coordinates
    Color(PremulColor<Srgb>),
    MaskTexture(PremulColor<Srgb>),
    ColorTexture,
    // Viewport space coordinates, these have a 1<=>1 projection to the viewport
    ColorViewport(PremulColor<Srgb>),
    MaskTextureViewport(PremulColor<Srgb>),
    ColorTextureViewport,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct CanvasCoordinates;
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct ViewportCoordinates;

#[const_trait]
pub trait ApplyCoordinates {
    fn apply(kind: VertexKind) -> VertexKind;
}

impl const ApplyCoordinates for CanvasCoordinates {
    fn apply(kind: VertexKind) -> VertexKind {
        match kind {
            VertexKind::ColorViewport(rgba) => VertexKind::Color(rgba),
            VertexKind::MaskTextureViewport(rgba) => VertexKind::MaskTexture(rgba),
            VertexKind::ColorTextureViewport => VertexKind::ColorTexture,
            _ => kind,
        }
    }
}

impl const ApplyCoordinates for ViewportCoordinates {
    fn apply(kind: VertexKind) -> VertexKind {
        match kind {
            VertexKind::Color(rgba) => VertexKind::ColorViewport(rgba),
            VertexKind::MaskTexture(rgba) => VertexKind::MaskTextureViewport(rgba),
            VertexKind::ColorTexture => VertexKind::ColorTextureViewport,
            _ => kind,
        }
    }
}

impl VertexKind {
    #[inline]
    pub const fn kind_id(&self) -> u32 {
        match self {
            VertexKind::Color(_) => 0,
            VertexKind::MaskTexture(_) => 1,
            VertexKind::ColorTexture => 2,
            VertexKind::ColorViewport(_) => 3,
            VertexKind::MaskTextureViewport(_) => 4,
            VertexKind::ColorTextureViewport => 5,
        }
    }
    #[inline]
    pub const fn color(&self) -> PremulColor<Srgb> {
        match self {
            VertexKind::Color(color) => *color,
            VertexKind::MaskTexture(color) => *color,
            VertexKind::ColorTexture => PremulColor::new([1., 0., 0., 1.]),
            VertexKind::ColorViewport(color) => *color,
            VertexKind::MaskTextureViewport(color) => *color,
            VertexKind::ColorTextureViewport => PremulColor::new([1., 0., 0., 1.]),
        }
    }
}

/// A generic vertex type for rendering.
#[repr(C)]
#[derive(Copy, Clone, Debug, Deserialize, Serialize, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
    pub kind: u32,
    pub texture: u32,
    pub tex_coords: [f32; 2],
}

impl Vertex {
    const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![0 => Float32x2, 1=> Float32x4, 2=> Uint32, 3=> Uint32, 4=> Float32x2];
    pub const fn buffer_layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::VERTEX_ATTRIBUTES,
        }
    }
    pub const fn with_color(position: Point, kind: VertexKind) -> Self {
        Self {
            position: [position.x, position.y],
            color: kind.color().components,
            kind: kind.kind_id(),
            texture: 0,
            tex_coords: [0., 0.],
        }
    }
}

impl Vertex {
    const fn from_texture_vertex(vertex: TextureVertex, kind: VertexKind) -> Self {
        Self {
            position: vertex.position,
            color: kind.color().components,
            kind: kind.kind_id(),
            texture: vertex.texture_layer,
            tex_coords: vertex.texture_coords,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq)]
pub struct Rounding {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_left: f32,
    pub bottom_right: f32,
}
impl Rounding {
    pub const DEFAULT: Self = Self {
        top_left: 0.,
        top_right: 0.,
        bottom_left: 0.,
        bottom_right: 0.,
    };
    pub const fn all(value: f32) -> Self {
        Self {
            top_left: value,
            top_right: value,
            bottom_left: value,
            bottom_right: value,
        }
    }
    pub const fn to_lyon(&self) -> BorderRadii {
        BorderRadii {
            top_left: self.top_left,
            top_right: self.top_right,
            bottom_left: self.bottom_left,
            bottom_right: self.bottom_right,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoxSizing {
    #[default]
    /// Box size includes the border
    BorderBox,
    /// Box size excludes the border
    ContentBox,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct BaseGradient {
    pub stops: Vec<GradientStop<PremulColor<Srgb>>>,

    spread: SpreadMethod,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GradientStop<T: Copy> {
    pub percent: f32,
    pub color: T,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpreadMethod {
    Pad,
    Reflect,
    Repeat,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RadialGradient {
    pub(crate) base: BaseGradient,

    /// Center point of the circle.
    pub(crate) center_point: Point2D<f32>,
    /// Where the gradient starts
    pub(crate) focal_point: Point2D<f32>,
    /// X and Y radius of the gradient.
    pub(crate) radius: Vector2D<f32>,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BasicLinearGradient {
    pub(crate) start_color: PremulColor<Srgb>,
    pub(crate) end_color: PremulColor<Srgb>,

    pub(crate) p1: Point2D<f32>,
    pub(crate) p2: Point2D<f32>,

    pub(crate) spread: SpreadMethod,
}
impl BasicLinearGradient {
    pub const fn new(start_color: PremulColor<Srgb>, end_color: PremulColor<Srgb>) -> Self {
        Self {
            start_color,
            end_color,
            p1: Point2D::new(0., 0.),
            p2: Point2D::new(0., 0.),
            spread: SpreadMethod::Pad,
        }
    }
    pub const fn new_with_points(
        start_color: PremulColor<Srgb>,
        end_color: PremulColor<Srgb>,
        p1: Point2D<f32>,
        p2: Point2D<f32>,
        spread_method: SpreadMethod,
    ) -> Self {
        Self {
            start_color,
            end_color,
            p1,
            p2,
            spread: spread_method,
        }
    }
    pub const fn update_points(&mut self, p1: Point2D<f32>, p2: Point2D<f32>) {
        self.p1 = p1;
        self.p2 = p2;
    }

    pub const fn as_color(self) -> BasicColor {
        BasicColor::LinearGradient(self)
    }

    pub fn get_point(&self, point: Point2D<f32>) -> PremulColor<Srgb> {
        // Vector from start to end
        let direction = self.p2 - self.p1;
        // Squared length of the gradient vector
        let length_sq = direction.square_length();

        // Avoid division by zero for degenerate segment
        if length_sq == 0.0 {
            tracing::warn!("get_point called with zero length");
            tracing::warn!("p1: {:?}, p2: {:?}", self.p1, self.p2);
            return self.start_color;
        }

        // Vector from p1 to the query point
        let to_point = point - self.p1;

        // Project to_point onto direction vector to find normalized parameter t
        let mut t = (to_point.x * direction.x + to_point.y * direction.y) / length_sq;

        // Apply spread method
        match self.spread {
            SpreadMethod::Pad => {
                t = t.clamp(0., 1.);
            }
            SpreadMethod::Repeat => {
                t = t.fract();
                if t < 0.0 {
                    t += 1.0;
                }
            }
            SpreadMethod::Reflect => {
                t = t.abs() % 2.0;
                if t > 1.0 {
                    t = 2.0 - t;
                }
            }
        };

        self.start_color
            .lerp(self.end_color, t, HueDirection::Shorter)
    }

    pub fn lerp(&self, other: Self, t: f32) -> Self {
        Self {
            start_color: self.start_color.lerp_rect(other.start_color, t),
            end_color: self.end_color.lerp_rect(other.end_color, t),
            p1: self.p1.lerp(other.p1, t),
            p2: self.p2.lerp(other.p2, t),
            spread: if t < 1. { self.spread } else { other.spread },
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BasicColor {
    Solid(PremulColor<Srgb>),
    LinearGradient(BasicLinearGradient),
}

impl BasicColor {
    pub fn lerp(&self, other: Self, t: f32) -> Self {
        match self {
            BasicColor::Solid(color) => match other {
                BasicColor::Solid(color2) => color.lerp_rect(color2, t).into(),
                BasicColor::LinearGradient(gradient) => BasicLinearGradient {
                    start_color: color.lerp_rect(gradient.start_color, t),
                    end_color: color.lerp_rect(gradient.end_color, t),
                    p1: gradient.p1,
                    p2: gradient.p2,
                    spread: gradient.spread,
                }
                .into(),
            },
            BasicColor::LinearGradient(gradient) => match other {
                BasicColor::Solid(color) => {
                    if t < 1. {
                        BasicLinearGradient {
                            start_color: gradient.start_color.lerp_rect(color, t),
                            end_color: gradient.end_color.lerp_rect(color, t),
                            p1: gradient.p1,
                            p2: gradient.p2,
                            spread: gradient.spread,
                        }
                        .into()
                    } else {
                        other
                    }
                }
                BasicColor::LinearGradient(gradient2) => gradient.lerp(gradient2, t).into(),
            },
        }
    }
}

impl From<PremulColor<Srgb>> for BasicColor {
    fn from(value: PremulColor<Srgb>) -> Self {
        Self::Solid(value)
    }
}
impl From<BasicLinearGradient> for BasicColor {
    fn from(value: BasicLinearGradient) -> Self {
        Self::LinearGradient(value)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ComplexColor {
    RadialGradient(RadialGradient),
}

pub const fn make_positive_box(mut area: Box2D<f32>) -> Box2D<f32> {
    if area.min.x > area.max.x {
        std::mem::swap(&mut area.min.x, &mut area.max.x);
    };
    if area.min.y > area.max.y {
        std::mem::swap(&mut area.min.y, &mut area.max.y);
    };
    area
}

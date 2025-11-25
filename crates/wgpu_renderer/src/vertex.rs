use atlas::TextureVertex;
use bytemuck::{Pod, Zeroable};
use color::{PremulColor, Srgb};
use euclid::default::Point2D;
use graphics_v2::BasicLinearGradient;

#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
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
}

#[repr(u32)]
#[derive(Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum VertexKind {
    Color = 0,
    MaskTexture = 1,
    ColorTexture = 2,
}

impl Vertex {
    #[inline(always)]
    pub const fn new_color(position: [f32; 2], color: PremulColor<Srgb>) -> Self {
        Self {
            position,
            color: color.components,
            kind: VertexKind::Color as u32,
            texture: 0,
            tex_coords: [0., 0.],
        }
    }
    #[inline(always)]
    pub const fn new_mask_texture(
        position: [f32; 2],
        color: PremulColor<Srgb>,
        texture: u32,
        tex_coords: [f32; 2],
    ) -> Self {
        Self {
            position,
            color: color.components,
            kind: VertexKind::MaskTexture as u32,
            texture,
            tex_coords,
        }
    }
    #[inline(always)]
    pub const fn new_color_texture(position: [f32; 2], texture: u32, tex_coords: [f32; 2]) -> Self {
        Self {
            position,
            color: [0., 0., 0., 1.],
            kind: VertexKind::ColorTexture as u32,
            texture,
            tex_coords,
        }
    }

    /// Create the vertices needed for a solid single color rectangle with min/max coordinates in
    /// CCW order. indices: 0,1,2 0,2,3
    #[inline(always)]
    pub const fn new_solid_rect(
        min: [f32; 2],
        max: [f32; 2],
        color: PremulColor<Srgb>,
    ) -> [Self; 4] {
        [
            Self {
                position: [max[0], min[1]],
                color: color.components,
                kind: VertexKind::Color as u32,
                texture: 0,
                tex_coords: [0., 0.],
            },
            Self {
                position: min,
                color: color.components,
                kind: VertexKind::Color as u32,
                texture: 0,
                tex_coords: [0., 0.],
            },
            Self {
                position: [min[0], max[1]],
                color: color.components,
                kind: VertexKind::Color as u32,
                texture: 0,
                tex_coords: [0., 0.],
            },
            Self {
                position: max,
                color: color.components,
                kind: VertexKind::Color as u32,
                texture: 0,
                tex_coords: [0., 0.],
            },
        ]
    }

    /// Create the vertices needed for a linear gradient rectangle with min/max coordinates in
    /// CCW order. indices: 0,1,2 0,2,3
    #[inline(always)]
    pub fn new_gradient_rect(
        min: [f32; 2],
        max: [f32; 2],
        color: &BasicLinearGradient,
    ) -> [Self; 4] {
        [
            Self {
                position: [max[0], min[1]],
                color: color
                    .get_point(Point2D::new(max[0], min[1]))
                    .premultiply()
                    .components,
                kind: VertexKind::Color as u32,
                texture: 0,
                tex_coords: [0., 0.],
            },
            Self {
                position: min,
                color: color
                    .get_point(Point2D::new(min[0], min[1]))
                    .premultiply()
                    .components,
                kind: VertexKind::Color as u32,
                texture: 0,
                tex_coords: [0., 0.],
            },
            Self {
                position: [min[0], max[1]],
                color: color
                    .get_point(Point2D::new(min[0], max[1]))
                    .premultiply()
                    .components,
                kind: VertexKind::Color as u32,
                texture: 0,
                tex_coords: [0., 0.],
            },
            Self {
                position: max,
                color: color
                    .get_point(Point2D::new(max[0], max[1]))
                    .premultiply()
                    .components,
                kind: VertexKind::Color as u32,
                texture: 0,
                tex_coords: [0., 0.],
            },
        ]
    }

    #[inline(always)]
    pub const fn from_texture_vertex(
        vertex: TextureVertex,
        color: PremulColor<Srgb>,
        kind: VertexKind,
    ) -> Self {
        Self {
            position: vertex.position,
            color: color.components,
            kind: kind as u32,
            texture: vertex.texture_layer,
            tex_coords: vertex.texture_coords,
        }
    }
}

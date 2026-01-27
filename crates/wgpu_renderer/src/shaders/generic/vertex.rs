use atlas::TextureVertex;
use color::{LinearSrgb, PremulColor};
use euclid::default::Point2D;
use graphics::BasicLinearGradient;

#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
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
        debug_assert!(
            size_of::<Self>().is_multiple_of(wgpu::VERTEX_ALIGNMENT as usize),
            "vertex alignment is not aligned to wgpu::VERTEX_ALIGNMENT bytes",
        );
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
    pub const fn new_color(position: [f32; 2], color: PremulColor<LinearSrgb>) -> Self {
        Self {
            position,
            color: color.components,
            kind: VertexKind::Color as u32,
            texture: 0,
            tex_coords: [0., 0.],
        }
    }
    /// Create the vertices needed for a solid single color rectangle with min/max coordinates in
    /// CCW order. indices: 0,1,2 0,2,3
    #[inline(always)]
    pub const fn new_solid_rect(
        min: [f32; 2],
        max: [f32; 2],
        color: PremulColor<LinearSrgb>,
    ) -> [Self; 4] {
        [
            Self::new_color([max[0], min[1]], color),
            Self::new_color(min, color),
            Self::new_color([min[0], max[1]], color),
            Self::new_color(max, color),
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
            Self::new_color(
                [max[0], min[1]],
                color.get_point_premul_cs(Point2D::new(max[0], min[1])),
            ),
            Self::new_color(min, color.get_point_premul_cs(Point2D::new(min[0], min[1]))),
            Self::new_color(
                [min[0], max[1]],
                color.get_point_premul_cs(Point2D::new(min[0], min[1])),
            ),
            Self::new_color(max, color.get_point_premul_cs(Point2D::new(max[0], max[1]))),
        ]
    }

    #[inline(always)]
    pub const fn from_texture_vertex(
        vertex: TextureVertex,
        color: PremulColor<LinearSrgb>,
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

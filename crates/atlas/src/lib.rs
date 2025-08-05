use anyhow::Result;
use euclid::default::{Box2D, Point2D, Size2D};
use rustc_hash::FxHasher;
use std::{collections::HashMap, hash::BuildHasherDefault, marker::PhantomData, sync::Arc};

use wgpu::{
    CommandBuffer, CommandEncoderDescriptor, Device, Extent3d, Limits, Origin3d,
    TexelCopyBufferLayout, TexelCopyTextureInfo, Texture, TextureAspect, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
};

use guillotiere::{AllocId, AtlasAllocator, Rectangle, Size};

type Hasher = BuildHasherDefault<FxHasher>;

pub mod backend;

pub const DEFAULT_TILE_SIZE: u32 = 128;
pub const DEFAULT_ATLAS_SIZE: Size = Size::new(2048, 2048);

pub struct UnallocatedTexture<'a> {
    pub data: &'a [u8],
    pub width: u32,
    pub height: u32,
}
impl<'a> UnallocatedTexture<'a> {
    pub fn new(data: &'a [u8], width: u32, height: u32) -> Self {
        Self {
            data,
            width,
            height,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AllocatedTexture<T: AtlasFormat, D> {
    tiles: Vec<AllocatedTile>,
    tile_size: u32,

    width: u32,
    height: u32,

    pub data: D,

    _marker: PhantomData<T>,
}

impl<T: AtlasFormat, D> AllocatedTexture<T, D> {
    pub fn to_mesh<K: std::hash::Hash + Eq + Clone>(
        &self,
        area: Box2D<f32>,
        atlas: &LayeredAtlas<T, K, D>,
    ) -> TextureMesh {
        let mut mesh = TextureMesh::empty();
        if area.is_empty() {
            return mesh;
        }

        let output_size = area.size();

        let area_ratio_w = self.width as f32 / output_size.width;
        let area_ratio_h = self.height as f32 / output_size.height;

        for tile in &self.tiles {
            // we use this instead of self.tile_size since we support non-standard tiles for the
            // edges of an texture e.g. with a tile size of 3
            // --------------
            // 111 | 111 | 11
            // 111 | 111 | 11 <- this tile is 2x3
            // 111 | 111 | 11
            // --------------
            // 111 | 111 | 11 <- this tile is 2x1
            // ^-----^
            // these tile are 3x1
            //
            // since real_tile_size will always equal self.tile_size (apart from the edges), it is
            // fine to use it for the second pair of coordinates.
            let real_tile_size = tile.location.rect.to_f32().size();

            let x1 = area.min.x + ((tile.column * self.tile_size) as f32 * area_ratio_w);
            let y1 = area.min.y + ((tile.row * self.tile_size) as f32 * area_ratio_h);

            let x2 = x1 + (real_tile_size.width * area_ratio_w);
            let y2 = y1 + (real_tile_size.height * area_ratio_h);

            let tile_output = Box2D {
                min: Point2D::<f32>::new(x1, y1),
                max: Point2D::<f32>::new(x2, y2),
            };

            let tile_block = tile.to_quad(tile_output, atlas);
            mesh.append_block(tile_block);
        }
        mesh
    }
}
pub struct UnallocatedTile<T: AtlasFormat> {
    data: Vec<u8>,

    column: u32,
    row: u32,

    width: u32,
    height: u32,

    _marker: PhantomData<T>,
}
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
struct AllocatedTile {
    column: u32,
    row: u32,
    location: AtlasLocation,
}
impl AllocatedTile {
    fn to_quad<T: AtlasFormat, K: std::hash::Hash + Eq + Clone, D>(
        &self,
        output_area: Box2D<f32>,
        atlas: &LayeredAtlas<T, K, D>,
    ) -> [TextureVertex; 4] {
        let atlas_size = atlas.size.to_f32();
        let min = self.location.rect.min.to_f32();
        let max = self.location.rect.max.to_f32();

        let start_uv = (min.x / atlas_size.width, min.y / atlas_size.height);
        let end_uv = (max.x / atlas_size.width, max.y / atlas_size.height);

        TextureVertex::new_block(output_area, self.location.layer, start_uv, end_uv)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AtlasLocation {
    pub layer: u32,

    pub id: AllocId,
    pub rect: Rectangle,
}

pub struct LayeredAtlas<T: AtlasFormat, K: std::hash::Hash + Eq + Clone, D> {
    layers: Vec<AtlasAllocator>,
    allocations: HashMap<Option<K>, Arc<AllocatedTexture<T, D>>, Hasher>,

    size: Size,
    /// When allocating an area greater than this (e.g. tile_size x tile_size), split it into
    /// chunks of size at most tile_size, this allows us to pack large textures tighter.
    tile_size: u32,

    texture: Texture,
    pub texture_view: TextureView,

    /// The max size that an individual texture can become.
    max_size: Size,
    max_layers: u32,

    pub needs_rebinding: bool,
    _marker: PhantomData<T>,
}

impl<T: AtlasFormat, K: std::hash::Hash + Eq + Clone, D> LayeredAtlas<T, K, D> {
    pub fn new(device: &Device, size: Size, tile_size: u32, limits: Limits) -> Self {
        let max_size = Size::splat(limits.max_texture_dimension_2d as i32);
        let max_layers = limits.max_texture_array_layers as u32;

        let size = size.min(max_size);

        let atlas = AtlasAllocator::new(size.min(max_size));

        let texture = Self::create_texture(device, 1, size.to_u32());
        let texture_view = texture.create_view(&TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });

        assert!(
            T::format().is_multi_planar_format() == false,
            "Can only create atlas for non-planar texture formats"
        );

        Self {
            layers: vec![atlas],
            allocations: HashMap::with_hasher(Hasher::new()),

            size,
            tile_size,

            texture,
            texture_view,

            max_size,
            max_layers,

            needs_rebinding: false,

            _marker: PhantomData,
        }
    }

    pub fn create_texture(device: &Device, layer_count: u32, size: Size2D<u32>) -> Texture {
        device.create_texture(&TextureDescriptor {
            label: Some("texture atlas"),
            size: Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: layer_count,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: T::format(),
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::COPY_SRC,
            view_formats: &[],
        })
    }

    pub fn copy_texture(
        device: &Device,
        layers: u32,
        size: Size2D<u32>,
        src: &Texture,
        dst: &Texture,
    ) -> CommandBuffer {
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("copy atlas texture"),
        });
        for layer in 0..layers {
            encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &src,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: layer,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyTextureInfo {
                    texture: &dst,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: layer,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d {
                    width: size.width,
                    height: size.height,
                    depth_or_array_layers: 1,
                },
            );
        }
        encoder.finish()
    }

    pub fn is_allocated(&mut self, key: K) -> Option<Arc<AllocatedTexture<T, D>>> {
        self.allocations.get(&Some(key)).map(|x| x.clone())
    }

    #[must_use]
    /// Allocates a texture onto one of the atlases, this returns an allocated texture reference.
    /// The allocated texture will be marked for deallocation when all references to it are dropped.
    ///
    /// If a texture is found in the atlas (using the cache key), it returns a reference to that
    /// allocated texture instead of trying to reallocate. If you wish to forcibly reallocate,
    pub fn allocate(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture: UnallocatedTexture,
        cache_key: Option<K>,
        data: D,
    ) -> Result<Arc<AllocatedTexture<T, D>>> {
        if let Some(cache_key) = &cache_key {
            if let Some(allocation) = self.is_allocated(cache_key.clone()) {
                return Ok(allocation);
            };
        }

        let (width, height) = (texture.width, texture.height);

        let unallocated_tiles = self.tile_texture(texture);

        let mut allocated_tiles = vec![];

        for tile in unallocated_tiles {
            allocated_tiles.push(self.allocate_tile(device, queue, tile)?);
        }

        let allocation = Arc::new(AllocatedTexture {
            tiles: allocated_tiles,
            tile_size: self.tile_size,

            width,
            height,

            data,

            _marker: PhantomData,
        });

        self.allocations.insert(cache_key, allocation.clone());
        Ok(allocation)
    }

    #[must_use]
    fn allocate_tile(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        tile: UnallocatedTile<T>,
    ) -> Result<AllocatedTile> {
        // try allocate in existing layers
        for (layer, atlas) in self.layers.iter_mut().enumerate() {
            if let Some(area) = atlas.allocate(Size::new(tile.width as i32, tile.height as i32)) {
                queue.write_texture(
                    TexelCopyTextureInfo {
                        texture: &self.texture,
                        mip_level: 0,
                        origin: Origin3d {
                            x: area.rectangle.min.x as u32,
                            y: area.rectangle.min.y as u32,
                            z: layer as u32,
                        },
                        aspect: TextureAspect::All,
                    },
                    tile.data.as_ref(),
                    TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(tile.width * T::format().components() as u32),
                        rows_per_image: None,
                    },
                    Extent3d {
                        width: tile.width,
                        height: tile.height,
                        depth_or_array_layers: 1,
                    },
                );
                return Ok(AllocatedTile {
                    column: tile.column,
                    row: tile.row,
                    location: AtlasLocation {
                        layer: layer as u32,
                        id: area.id,
                        rect: area.rectangle,
                    },
                });
            }
        }
        if self.layers.len() < self.max_layers as usize {
            let size = self.size.to_u32();
            let new_texture = Self::create_texture(device, self.layers.len() as u32 + 1, size);
            let command_buffer = Self::copy_texture(
                device,
                self.layers.len() as u32,
                size,
                &self.texture,
                &new_texture,
            );
            queue.submit(Some(command_buffer));
            self.layers.push(AtlasAllocator::new(self.size));
            self.texture.destroy();
            self.texture = new_texture;
            self.texture_view = self.texture.create_view(&TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                ..Default::default()
            });
            self.needs_rebinding = true;
            return self.allocate_tile(device, queue, tile);
        }
        if self.size.area() < self.max_size.area() {
            let size = self.size.to_u32();
            let new_size = size * 2;
            let new_texture = Self::create_texture(device, self.layers.len() as u32, new_size);
            let command_buffer = Self::copy_texture(
                device,
                self.layers.len() as u32,
                size,
                &self.texture,
                &new_texture,
            );
            queue.submit(Some(command_buffer));
            self.texture.destroy();
            self.texture = new_texture;
            self.texture_view = self.texture.create_view(&TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                ..Default::default()
            });
            self.needs_rebinding = true;
            return self.allocate_tile(device, queue, tile);
        }
        // extend texture to have more layers
        // TODO: replace with thiserror
        anyhow::bail!(
            "failed to allocate the given tile with dimensions: {}x{}",
            tile.width,
            tile.height
        );
    }

    #[must_use]
    pub fn allocate_raw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        contents: Vec<u8>,
        width: u32,
        height: u32,
        cache_key: Option<K>,
        data: D,
    ) -> Result<Arc<AllocatedTexture<T, D>>> {
        if let Some(cache_key) = &cache_key {
            if let Some(allocation) = self.is_allocated(cache_key.clone()) {
                return Ok(allocation);
            };
        }

        let tile = self.allocate_tile(
            device,
            queue,
            UnallocatedTile {
                data: contents,
                column: 0,
                row: 0,
                width,
                height,
                _marker: PhantomData,
            },
        )?;
        let allocated = Arc::new(AllocatedTexture {
            tiles: vec![tile],
            tile_size: self.tile_size,
            width,
            height,

            data,

            _marker: PhantomData,
        });
        self.allocations.insert(cache_key, allocated.clone());
        Ok(allocated)
    }

    pub fn tile_texture(&self, texture: UnallocatedTexture) -> Vec<UnallocatedTile<T>> {
        let mut tiles: Vec<UnallocatedTile<T>> = Vec::new();
        let rows = (texture.height + self.tile_size - 1) / self.tile_size;
        let cols = (texture.width + self.tile_size - 1) / self.tile_size;
        tiles.reserve_exact((rows * cols) as usize);

        let stride = T::format().components() as u32;

        for row in 0..rows {
            for column in 0..cols {
                let x = column * self.tile_size;
                let y = row * self.tile_size;

                let tile_width = (texture.width - x).min(self.tile_size);
                let tile_height = (texture.height - y).min(self.tile_size);

                let mut tile_data = Vec::new();
                tile_data.reserve_exact((tile_width * tile_height) as usize);

                for ty in 0..tile_height {
                    let src_start = ((y + ty) * texture.width + x) * stride;
                    let src_end = src_start + tile_width * stride;
                    tile_data
                        .extend_from_slice(&texture.data[src_start as usize..src_end as usize]);
                }

                tiles.push(UnallocatedTile {
                    data: tile_data,

                    column,
                    row,

                    width: tile_width,
                    height: tile_height,

                    _marker: PhantomData,
                });
            }
        }

        tiles
    }
    pub fn deallocate(&mut self) {
        tracing::trace!("calling deallocate on atlas allocation");
        // remove old texture
        let (alive, dropped): (HashMap<_, _, _>, HashMap<_, _, _>) = self
            .allocations
            .drain()
            .partition(|(_, texture)| Arc::strong_count(&texture) > 1);
        self.allocations = alive;

        for (_key, texture) in &dropped {
            for tile in &texture.tiles {
                if let Some(atlas) = self.layers.get_mut(tile.location.layer as usize) {
                    atlas.deallocate(tile.location.id)
                };
            }
        }
    }
    // TODO: rearrange, try remove layers, upgrade texture allocations references and update their
    // tiles
    pub fn rearrange(&mut self) {}
}

pub mod formats {
    #[derive(Clone, Copy, Debug)]
    pub struct Rgba8;
    #[derive(Clone, Copy, Debug)]
    pub struct Mask;
}
pub trait AtlasFormat {
    fn format() -> TextureFormat;
}
impl AtlasFormat for formats::Mask {
    fn format() -> TextureFormat {
        TextureFormat::R8Unorm
    }
}
impl AtlasFormat for formats::Rgba8 {
    fn format() -> TextureFormat {
        TextureFormat::Rgba8UnormSrgb
    }
}

#[derive(Clone, Debug)]
pub struct TextureVertex {
    pub position: [f32; 2],
    pub texture_layer: u32,
    pub texture_coords: [f32; 2],
}

#[derive(Default, Clone, Debug)]
pub struct TextureMesh {
    pub vertices: Vec<TextureVertex>,
    pub indices: Vec<u32>,
}
impl TextureMesh {
    pub fn empty() -> Self {
        Self::default()
    }
    pub fn append(&mut self, other: TextureMesh) {
        let offset = self.vertices.len() as u32;
        self.vertices.extend(other.vertices);
        // TODO: move this to offset_indices
        self.indices
            .extend(other.indices.iter().map(|i| offset + i));
    }

    fn append_block(&mut self, vertices: [TextureVertex; 4]) {
        let offset = self.vertices.len() as u32;
        self.vertices.extend(vertices);
        self.indices
            .extend(TextureVertex::BLOCK_INDICES.iter().map(|i| offset + i));
    }
}

// TODO: move this
impl TextureVertex {
    const BLOCK_INDICES: [u32; 6] = [0, 1, 2, 0, 2, 3];
    fn new_block(
        area: Box2D<f32>,
        texture_layer: u32,
        start_uv: (f32, f32),
        end_uv: (f32, f32),
    ) -> [TextureVertex; 4] {
        [
            TextureVertex {
                position: [area.min.x, area.min.y],
                texture_layer,
                texture_coords: [start_uv.0, start_uv.1],
            },
            TextureVertex {
                position: [area.max.x, area.min.y],
                texture_layer,
                texture_coords: [end_uv.0, start_uv.1],
            },
            TextureVertex {
                position: [area.max.x, area.max.y],
                texture_layer,
                texture_coords: [end_uv.0, end_uv.1],
            },
            TextureVertex {
                position: [area.min.x, area.max.y],
                texture_layer,
                texture_coords: [start_uv.0, end_uv.1],
            },
        ]
    }
}

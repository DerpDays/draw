use std::hash::{DefaultHasher, Hash, Hasher};

use crate::{
    CacheKey,
    GraphicsContext,
    Mesh,
    PrimitiveCache,
    TextureData,
    shaders::generic::Vertex,
};
use atlas::UnallocatedTexture;
use color::{AlphaColor, PremulColor, Rgba8, Srgb};
use euclid::Box2D;
use graphics::primitives::Svg;
use resvg::tiny_skia;

#[derive(Hash)]
// bit representation of f32
struct SvgHash<'a> {
    data: &'a [u8],
    size: [u32; 2],
    fill_color: Option<[u32; 4]>,
    stroke_color: Option<[u32; 4]>,
}
impl<'a> SvgHash<'a> {
    #[profiling::function]
    pub fn hash(
        data: &'a [u8],
        size: &euclid::default::Size2D<f32>,
        fill_color: Option<AlphaColor<Srgb>>,
        stroke_color: Option<AlphaColor<Srgb>>,
    ) -> u64 {
        let mut hasher = DefaultHasher::new();
        Self {
            data,
            size: size.to_array().map(|x| x.to_bits()),
            fill_color: fill_color.map(|x| x.components.map(|x| x.to_bits())),
            stroke_color: stroke_color.map(|x| x.components.map(|x| x.to_bits())),
        }
        .hash(&mut hasher);
        hasher.finish()
    }
}

#[profiling::function]
pub fn render_svg(
    ctx: &mut GraphicsContext,
    svg: &Svg,
    cache: &mut Option<PrimitiveCache>,
) -> Mesh<Vertex> {
    let hash = SvgHash::hash(&svg.data, &svg.size, svg.fill_color, svg.stroke_color);
    let cache_key = CacheKey::Hash(hash);

    let allocated_texture = if let Some(allocated_texture) = ctx
        .texture_state
        .color_atlas
        .is_allocated(cache_key.clone())
    {
        allocated_texture
    } else {
        let style_sheet = stylesheet(svg.fill_color, svg.stroke_color);
        let tree = {
            let opt = usvg::Options {
                style_sheet: Some(style_sheet),
                ..usvg::Options::default()
            };
            // opt.fontdb_mut().load_system_fonts();

            usvg::Tree::from_data(&svg.data, &opt).unwrap()
        };
        //
        let transform = usvg::Transform::from_scale(
            svg.size.width.floor() / tree.size().width(),
            svg.size.height.floor() / tree.size().height(),
        );
        let pixmap = tiny_skia::Pixmap::new(svg.size.width as u32, svg.size.height as u32);
        let Some(mut pixmap) = pixmap else {
            log::warn!("Tried to render a svg with size 0x0");
            return Mesh::empty();
        };
        resvg::render(&tree, transform, &mut pixmap.as_mut());

        let texture =
            UnallocatedTexture::new(pixmap.data(), svg.size.width as u32, svg.size.height as u32);
        ctx.texture_state
            .color_atlas
            .allocate(
                &ctx.device,
                &ctx.queue,
                texture,
                Some(cache_key),
                TextureData::None,
            )
            .unwrap()
    };

    let texture_mesh = allocated_texture.to_mesh(
        Box2D::from_origin_and_size(svg.origin, svg.size),
        &ctx.texture_state.color_atlas,
    );
    *cache = Some(PrimitiveCache {
        mask_textures: vec![],
        color_textures: vec![allocated_texture],
    });

    Mesh {
        vertices: texture_mesh
            .vertices
            .into_iter()
            .map(|vert| {
                Vertex::from_texture_vertex(
                    vert,
                    PremulColor::TRANSPARENT,
                    crate::shaders::generic::VertexKind::ColorTexture,
                )
            })
            .collect(),
        indices: texture_mesh.indices,
    }
}

fn stylesheet(fill: Option<AlphaColor<Srgb>>, stroke: Option<AlphaColor<Srgb>>) -> String {
    format!(
        "* {{ {} {} }}",
        fill.map_or("".to_string(), |x| alpha_color_to_css("fill", x)),
        stroke.map_or("".to_string(), |x| alpha_color_to_css("stroke", x))
    )
}

fn alpha_color_to_css(rule: &str, color: AlphaColor<Srgb>) -> String {
    let Rgba8 { r, g, b, a } = color.to_rgba8();
    format!("{rule}: rgba({},{},{},{});", r, g, b, a)
}

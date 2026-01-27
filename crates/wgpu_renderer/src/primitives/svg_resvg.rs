use crate::{GraphicsContext, Mesh, PrimitiveCache, TextureData, shaders::generic::Vertex};
use atlas::UnallocatedTexture;
use color::{AlphaColor, PremulColor, Rgba8, Srgb};
use euclid::Box2D;
use graphics::{BasicColor, primitives::Svg};
use resvg::tiny_skia;

pub fn render_svg(
    ctx: &mut GraphicsContext,
    svg: &Svg,
    cache: &mut Option<PrimitiveCache>,
) -> Mesh<Vertex> {
    let style_sheet = stylesheet(
        svg.fill_color.map(|x| match x {
            BasicColor::Solid(color) => color,
            _ => todo!(),
        }),
        svg.stroke_color.map(|x| match x {
            BasicColor::Solid(color) => color,
            _ => todo!(),
        }),
    );
    let tree = {
        let mut opt = usvg::Options {
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
    let mut pixmap = tiny_skia::Pixmap::new(svg.size.width as u32, svg.size.height as u32).unwrap();
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let texture =
        UnallocatedTexture::new(pixmap.data(), svg.size.width as u32, svg.size.height as u32);
    let allocated_texture = ctx
        .texture_state
        .color_atlas
        .allocate(&ctx.device, &ctx.queue, texture, None, TextureData::None)
        .unwrap();

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

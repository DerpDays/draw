use euclid::default::{Box2D, Point2D, Size2D};
use graphics_v2::{Primitive, primitives::CustomPrimitive};
use gui_v2::{
    MeasureCtx,
    TreeManager,
    prelude::{AvailableSpace, Layout, MaybeDyn, Size, Style, create_effect},
    reexports::reactive::maybe_get_clone_untracked,
    tree::{Widget, builder::ElementBuilder},
};

use crate::shaders::texture::{TextureBindGroup, TexturePrimitive};

pub struct WgpuTexture {
    bind_group: MaybeDyn<TextureBindGroup>,
}
impl From<TextureBindGroup> for MaybeDyn<TextureBindGroup> {
    fn from(value: TextureBindGroup) -> Self {
        Self::Static(value)
    }
}

impl Widget for WgpuTexture {
    fn render(&mut self, layout: &Layout, _: &Style) -> Option<Primitive> {
        Some(Primitive::Custom(
            TexturePrimitive {
                bind_group: maybe_get_clone_untracked(&self.bind_group),
                area: Box2D::from_origin_and_size(
                    Point2D::new(layout.location.x, layout.location.y),
                    Size2D::new(layout.size.width, layout.size.height),
                ),
            }
            .clone_box(),
        ))
    }
    fn measure(
        &mut self,
        _: &mut dyn MeasureCtx,
        known_dimensions: Size<Option<f32>>,
        _: Size<AvailableSpace>,
        _: &Style,
    ) -> Size<f32> {
        known_dimensions.unwrap_or(Size::zero())
    }
    fn debug_label(&self) -> &'static str {
        "Button"
    }

    fn focusable(&self) -> bool {
        true
    }
}

pub fn wgpu_texture(
    bind_group: impl Into<MaybeDyn<TextureBindGroup>>,
) -> ElementBuilder<WgpuTexture> {
    let bind_group = bind_group.into();
    ElementBuilder::new_with_after_build(
        WgpuTexture {
            bind_group: bind_group.clone(),
        },
        move |_| {
            let mgr = TreeManager::global();
            let bind_group = bind_group.clone();

            create_effect(move || {
                bind_group.track();
                mgr.now();
            });
        },
    )
}

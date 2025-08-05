use anyhow::Result;
use input::sctk::KeyEventKind;
use smithay_client_toolkit::seat::pointer::{PointerEvent, ThemedPointer};
use tracing::instrument;
use wayland_client::protocol::wl_surface::WlSurface;

mod canvas;

pub use canvas::LayerShellCanvasView;

use crate::ShareableState;

pub struct ViewManager {
    pub layer_shell_views: Vec<LayerShellCanvasView>,
}

impl ViewManager {
    #[instrument(name = "ViewManager::add_canvas_view", skip_all)]
    pub fn add_canvas_view(&mut self, view: LayerShellCanvasView) {
        self.layer_shell_views.push(view)
    }

    pub fn from_surface(&mut self, surface: &WlSurface) -> &mut dyn View {
        for layer in &mut self.layer_shell_views {
            if layer.surface() == surface {
                return layer;
            }
        }
        // if self.toolbar_view.surface() == surface {
        //     return &mut self.toolbar_view;
        // }
        unreachable!("failed to find a view from the given surface");
    }

    pub fn canvas_views(&mut self) -> Vec<&mut LayerShellCanvasView> {
        self.layer_shell_views.iter_mut().collect::<Vec<_>>()
    }
}

pub trait View {
    fn render(&mut self, state: &mut ShareableState) -> Result<()>;
    fn surface(&self) -> &WlSurface;
    fn configure(&mut self, state: &mut ShareableState, width: u32, height: u32);

    fn set_scale_factor(&mut self, state: &mut ShareableState, scale_factor: f64);
    fn get_scale_factor(&self) -> Option<f64>;

    fn pointer_event(
        &mut self,
        state: &mut ShareableState,
        themed_pointer: &ThemedPointer,
        event: &PointerEvent,
    );

    fn keyboard_event(&mut self, state: &mut ShareableState, kind: &KeyEventKind);
}

pub trait LayerShellView: View {
    fn get_mode(&self) -> crate::OverlayMode;
    fn set_mode(&mut self, state: &mut ShareableState, mode: crate::OverlayMode) -> Result<()>;
}

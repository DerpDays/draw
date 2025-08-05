use std::{num::NonZero, ptr::NonNull};

use anyhow::{Context, Result};
use euclid::default::{Point2D, Size2D};
use input::{sctk::KeyEventKind, CursorIcon, MouseEvent, MouseEventKind};
use smithay_client_toolkit::{
    compositor::Region,
    seat::pointer::{PointerEvent, ThemedPointer},
    shell::{
        wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerSurface},
        WaylandSurface,
    },
};
use tracing::{info, instrument, trace, warn};
use wayland_client::{
    protocol::{wl_output::WlOutput, wl_surface::WlSurface},
    Proxy,
};

use crate::fractional_scale::FractionalScale;
use crate::{OverlayMode, RedrawManager, ShareableState};

use crate::views::{LayerShellView, View};

// Layer shell view implementation
pub struct LayerShellCanvasView {
    pub layer_surface: LayerSurface,
    pub fractional_scale: FractionalScale,
    pub wgpu_surface: wgpu::Surface<'static>,

    pub scale_factor: Option<f64>,
    pub size: Size2D<f64>,

    pub canvas: canvas::view::View<RedrawManager>,
    pub previous_cursor_icon: Option<CursorIcon>,

    pub mode: OverlayMode,
    pub configured: bool,
}

impl LayerShellCanvasView {
    #[instrument(name = "LayerShellCanvasView::new", skip_all)]
    pub async fn new(
        state: &mut ShareableState,
        output: &WlOutput,
        mode: OverlayMode,
    ) -> Result<Self> {
        trace!("Creating a surface for overlay canvas");
        let surface = state
            .wayland
            .compositor
            .create_surface(&state.wayland.queue_handle);
        // TODO: New overlay for each output
        let layer_surface = state.wayland.layer_shell.create_layer_surface(
            &state.wayland.queue_handle,
            surface,
            Layer::Overlay,
            Some("annotate"),
            None,
        );
        trace!("getting display size");

        let info = state.wayland.output_state.info(output).unwrap();
        trace!("Found info for: {:?}: {:?}", info.name, info.modes);
        tracing::info!("modes: {:?}", info.modes);
        let current_mode = info
            .modes
            .iter()
            .find(|mode| mode.current)
            .context("cannot determine output size: display has no current mode")?;

        let (width, height) = current_mode.dimensions;

        // TODO: niri gives these output sizes already scaled
        let uwidth: NonZero<u32> = u32::try_from(width)
            .context("display width must be positive")?
            .try_into()
            .context("display width must not be zero")?;
        let uheight: NonZero<u32> = u32::try_from(height)
            .context("display height must be positive")?
            .try_into()
            .context("display height must not be zero")?;

        let fractional_scale = state
            .wayland
            .fractional_state
            .get_scale(layer_surface.wl_surface(), &state.wayland.queue_handle);

        let canvas_viewport = Size2D::new(width as f64, height as f64);
        layer_surface.set_size(uwidth.into(), uheight.into());
        // initial commit before we attach wgpu to the surface.

        // INFO: WGPU stuff
        trace!("creating wgpu surface");
        let raw_display_handle =
            wgpu::rwh::RawDisplayHandle::Wayland(wgpu::rwh::WaylandDisplayHandle::new(
                NonNull::new(state.wayland.connection.backend().display_ptr() as *mut _).unwrap(),
            ));
        let raw_window_handle =
            wgpu::rwh::RawWindowHandle::Wayland(wgpu::rwh::WaylandWindowHandle::new(
                NonNull::new(layer_surface.wl_surface().id().as_ptr() as *mut _).unwrap(),
            ));
        let wgpu_surface = unsafe {
            state
                .wgpu
                .instance
                .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                    raw_display_handle,
                    raw_window_handle,
                })
                .expect("failed to create wgpu surface")
        };
        // Configure the layer surface, providing things like the anchor on screen, desired size and the keyboard
        // interactivity

        // Place the layer on top of all other layers.
        layer_surface.set_anchor(Anchor::TOP | Anchor::LEFT);
        layer_surface.set_exclusive_zone(-1);

        layer_surface.commit();
        let redraw_manager = RedrawManager::new(state.loop_handle.clone(), Some(240), 60);

        Ok(Self {
            layer_surface,
            wgpu_surface,
            fractional_scale,

            mode,
            size: canvas_viewport,
            canvas: canvas::view::View::new(&state.wgpu, canvas_viewport.cast(), redraw_manager),
            previous_cursor_icon: None,

            scale_factor: None,

            configured: false,
        })
    }
}

impl LayerShellView for LayerShellCanvasView {
    fn get_mode(&self) -> crate::OverlayMode {
        self.mode
    }
    #[instrument(name = "LayerShellCanvasView::set_mode", skip_all)]
    fn set_mode(&mut self, state: &mut ShareableState, mode: OverlayMode) -> Result<()> {
        info!("setting the mode to {mode:?}");
        self.mode = mode;
        match mode {
            OverlayMode::Interactive | OverlayMode::Keybind => {
                self.layer_surface
                    .set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
                self.layer_surface.set_input_region(None);
                self.layer_surface.commit();
            }
            OverlayMode::Visible | OverlayMode::Hidden => {
                self.layer_surface
                    .set_keyboard_interactivity(KeyboardInteractivity::None);
                let empty_region =
                    Region::new(&state.wayland.compositor).context("cannot make region")?;
                self.layer_surface
                    .set_input_region(Some(empty_region.wl_region()));
                tracing::info!("committing canvas surface!");
                self.layer_surface.commit();
            }
        };
        _ = self.render(state);
        Ok(())
    }
}

impl View for LayerShellCanvasView {
    #[instrument(name = "LayerShellCanvasView::render", skip_all)]
    fn render(&mut self, state: &mut ShareableState) -> Result<()> {
        if !self.configured {
            return Ok(());
        }
        trace!("rendering canvas view");
        if matches!(self.mode, OverlayMode::Hidden) {
            tracing::warn!("rendering in mode: Hidden");
            match self.wgpu_surface.get_current_texture() {
                Ok(frame) => {
                    let view = frame
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());
                    renderer::render_bg(&view, &state.wgpu, wgpu::Color::TRANSPARENT);
                    frame.present();
                }
                Err(error) => match error {
                    wgpu::SurfaceError::OutOfMemory => {
                        panic!(
                            "Swapchain error: {error}. \
                                Rendering cannot continue."
                        )
                    }
                    _ => {
                        // Try rendering again next frame.
                        _ = self.render(state);
                    }
                },
            }

            return Ok(());
        }

        self.canvas
            .render(&state.wgpu, &self.wgpu_surface, &state.app_pipeline);
        Ok(())
    }

    #[instrument(name = "LayerShellCanvasView::surface", skip(self))]
    fn surface(&self) -> &WlSurface {
        self.layer_surface.wl_surface()
    }

    #[instrument(name = "LayerShellCanvasView::configure", skip_all)]
    fn configure(&mut self, state: &mut ShareableState, width: u32, height: u32) {
        trace!("configuring canvas with size: {width}x{height}");
        self.wgpu_surface.configure(
            &state.wgpu.device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: state.wgpu.texture_format,
                width,
                height,
                present_mode: wgpu::PresentMode::Mailbox,
                desired_maximum_frame_latency: 0,
                alpha_mode: wgpu::CompositeAlphaMode::PreMultiplied,
                view_formats: vec![],
            },
        );
        let scale_factor = self.scale_factor.unwrap_or(1.);
        // if new_size != self.size {
        //     self.size = new_size;
        //
        //     self.set_scale_factor(state, self.scale_factor.unwrap_or(1.));
        // }
        if !self.configured {
            self.configured = true;
            _ = self.set_mode(state, self.mode);
        }
    }

    fn pointer_event(
        &mut self,
        state: &mut ShareableState,
        themed_pointer: &ThemedPointer,
        event: &PointerEvent,
    ) {
        let position = Point2D::new(event.position.0 as f32, event.position.1 as f32);
        // let position = lyon::math::point(
        //     (event.position.0 * self.scale_factor.unwrap_or(1.)) as f32,
        //     (event.position.1 * self.scale_factor.unwrap_or(1.)) as f32,
        // );
        let kind = input::sctk::pointer_event(&event.kind);
        if kind == MouseEventKind::Enter {
            self.previous_cursor_icon = None;
        }

        let cursor_icon = self
            .canvas
            .mouse_event(MouseEvent { position, kind }, &state.wgpu);

        if let Some(cursor_icon) = cursor_icon {
            if self.previous_cursor_icon != Some(cursor_icon) {
                self.previous_cursor_icon = Some(cursor_icon);
                _ = themed_pointer.set_cursor(
                    &state.wayland.connection,
                    input::sctk::cursor_icon(cursor_icon),
                );
            }
        };
    }

    fn keyboard_event(&mut self, state: &mut ShareableState, kind: &KeyEventKind) {
        match kind {
            KeyEventKind::Press((event, _modifiers)) => {
                tracing::warn!("char: {:?}", event.keysym.key_char());
            }
            _ => {}
        }

        let event = input::sctk::keyboard_event(&kind);
        self.canvas.keyboard_event(event, &state.wgpu);
    }
    fn set_scale_factor(&mut self, state: &mut ShareableState, scale_factor: f64) {
        tracing::warn!("scale factor for canvas: {scale_factor}");
        let new_scaled_size = (self.size / scale_factor).ceil();
        self.layer_surface
            .set_size(new_scaled_size.width as u32, new_scaled_size.height as u32);
        self.canvas.update_viewport(new_scaled_size.cast());
        self.layer_surface.commit();

        self.scale_factor = Some(scale_factor);
        _ = self.render(state);
    }

    fn get_scale_factor(&self) -> Option<f64> {
        self.scale_factor
    }
}

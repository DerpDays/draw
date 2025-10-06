use std::{ptr::NonNull, sync::Arc};

use color_eyre::eyre::{Context, OptionExt, Result};
use euclid::default::{Point2D, Size2D};
use input::{sctk::KeyEventKind, CursorIcon, MouseEvent, MouseEventKind};
use renderer::reexports::wgpu;
use smithay_client_toolkit::{
    compositor::Region,
    reexports::client::{
        protocol::{wl_output::WlOutput, wl_surface::WlSurface},
        Proxy,
    },
    seat::pointer::{PointerEvent, ThemedPointer},
    shell::{
        wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerSurface},
        WaylandSurface,
    },
};
use tracing::{info, trace};

use crate::wayland::{
    protocols::{FractionalScale, Viewport},
    OverlayMode,
    RedrawManager,
    RedrawManagerV2,
    ShareableState,
    WaylandState,
};

// Layer shell view implementation
pub struct LayerShellCanvasView {
    pub layer_surface: LayerSurface,
    _fractional_scale: FractionalScale,
    pub viewport: Viewport,
    pub wgpu_surface: wgpu::Surface<'static>,

    pub scale_factor: Option<f64>,
    pub physical_size: Size2D<u32>,

    pub canvas: canvas::view::View<RedrawManagerV2>,
    pub previous_cursor_icon: Option<CursorIcon>,

    pub mode: OverlayMode,
    pub configured: bool,
}

impl LayerShellCanvasView {
    pub fn new(state: &mut ShareableState, output: &WlOutput, mode: OverlayMode) -> Result<Self> {
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

        let (_, physical_size) =
            WaylandState::extract_display_info(&state.wayland.output_state, output)?;
        // let physical_size = Size2D::new(physical_size.0, physical_size.1);

        let fractional_scale = state
            .wayland
            .fractional_state
            .get_scale(layer_surface.wl_surface(), &state.wayland.queue_handle);

        let viewport = state
            .wayland
            .viewporter
            .get_viewport(layer_surface.wl_surface(), &state.wayland.queue_handle);
        viewport.set_destination(physical_size.width, physical_size.height);

        let physical_size = physical_size
            .try_cast::<u32>()
            .ok_or_eyre("output size must be positive")?;

        layer_surface.set_size(physical_size.width as u32, physical_size.height as u32);
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

        layer_surface.wl_surface().frame(
            &state.wayland.queue_handle,
            layer_surface.wl_surface().clone(),
        );
        layer_surface.wl_surface().commit();

        Ok(Self {
            layer_surface,
            wgpu_surface,
            viewport,
            _fractional_scale: fractional_scale,

            mode,
            physical_size,
            canvas: canvas::view::View::new(
                &state.wgpu,
                physical_size.cast(),
                1.,
                state.redraw_manager_v2.clone(),
            ),
            previous_cursor_icon: None,

            scale_factor: None,

            configured: false,
        })
    }
}

impl LayerShellCanvasView {
    pub fn set_mode(&mut self, state: &mut ShareableState, mode: OverlayMode) -> Result<()> {
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

impl LayerShellCanvasView {
    pub fn render(&mut self, state: &mut ShareableState) -> Result<()> {
        if !self.configured {
            return Ok(());
        }

        let start = std::time::Instant::now();
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
        tracing::trace!("Time taken to render: {:?}", start.elapsed());
        Ok(())
    }

    pub fn surface(&self) -> &WlSurface {
        self.layer_surface.wl_surface()
    }

    pub fn configure(&mut self, state: &mut ShareableState, width: u32, height: u32) {
        trace!(
            "configuring canvas with size: {width}x{height} : self.monitor_size: {:?}",
            self.physical_size
        );

        // let logical_size = (self.physical_size.cast() / self.get_scale_factor())
        //     .ceil()
        //     .cast();
        self.wgpu_surface.configure(
            &state.wgpu.device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: state.wgpu.texture_format,
                width: self.physical_size.width,
                height: self.physical_size.height,
                present_mode: wgpu::PresentMode::Mailbox,
                desired_maximum_frame_latency: 0,
                alpha_mode: wgpu::CompositeAlphaMode::PreMultiplied,
                view_formats: vec![],
            },
        );

        if !self.configured {
            trace!("first configure");
            self.configured = true;
            _ = self.set_mode(state, self.mode);
        }
    }

    pub fn pointer_event(
        &mut self,
        state: &mut ShareableState,
        themed_pointer: &ThemedPointer,
        event: &PointerEvent,
    ) {
        // let position = Point2D::new(event.position.0 as f32, event.position.1 as f32);
        let position: Point2D<f32> =
            (Point2D::from(event.position) * self.scale_factor.unwrap_or(1.)).cast();
        tracing::trace!("mouse pos: {position:?}");
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

    pub fn keyboard_event(&mut self, state: &mut ShareableState, kind: &KeyEventKind) {
        match kind {
            KeyEventKind::Press((event, _modifiers)) => {
                tracing::warn!("char: {:?}", event.keysym.key_char());
            }
            _ => {}
        }

        let event = input::sctk::keyboard_event(&kind);
        self.canvas.keyboard_event(event, &state.wgpu);
    }
    pub fn set_scale_factor(&mut self, state: &mut ShareableState, scale_factor: f64) {
        tracing::warn!("scale factor for canvas: {scale_factor}");
        let logical_size = (self.physical_size.cast() / scale_factor).ceil().cast();
        self.layer_surface
            .set_size(self.physical_size.width, self.physical_size.height);
        self.canvas
            .update_viewport(self.physical_size.cast(), self.get_scale_factor());
        self.viewport
            .set_destination(logical_size.width, logical_size.height);
        self.layer_surface.commit();

        self.scale_factor = Some(scale_factor);
        _ = self.render(state);
    }

    fn get_scale_factor(&self) -> f64 {
        self.scale_factor.unwrap_or(1.)
    }
}

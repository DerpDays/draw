use std::{
    cell::Cell,
    collections::HashMap,
    i32,
    rc::Rc,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use canvas::RedrawRequest;
use fractional_scale::FractionalScaleHandler;
use keyboard::Keyboard;
use tracing::{info, instrument, trace, warn};

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_pointer,
    delegate_registry, delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    reexports::{
        calloop::{
            self,
            timer::{TimeoutAction, Timer},
            EventLoop, LoopHandle,
        },
        calloop_wayland_source::WaylandSource,
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{pointer::ThemedPointer, SeatState},
    shell::wlr_layer::{LayerShell, LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
    shm::{Shm, ShmHandler},
};

use views::LayerShellView;
use wayland_backend::client::ObjectId;
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_output::WlOutput, wl_surface::WlSurface},
    Connection, EventQueue, QueueHandle,
};

pub mod clipboard;
pub mod fractional_scale;
pub mod global_binds;
mod keyboard;
mod pointer;
mod seat;
pub mod viewporter;
pub mod views;

use crate::global_binds::{ShortcutEvents, Shortcuts, WaylandKeybinds};
use crate::views::{View, ViewManager};

// FIXME: use this
#[allow(unused)]
pub fn is_wayland() -> bool {
    std::env::var("XDG_SESSION_TYPE").map_or_else(
        |_| std::env::var("WAYLAND_DISPLAY").is_ok(),
        |env| env == "wayland",
    )
}

pub struct WaylandConnection {
    event_queue: EventQueue<State>,
    event_loop: EventLoop<'static, State>,
    pub state: State,
}

pub struct State {
    pub shareable: ShareableState,

    pointers: HashMap<ObjectId, (ObjectId, ThemedPointer)>,
    keyboards: HashMap<ObjectId, Keyboard>,

    pub views: ViewManager,
}

pub struct WaylandState {
    connection: Connection,
    compositor: CompositorState,
    output_state: OutputState,
    queue_handle: QueueHandle<State>,
    registry_state: RegistryState,
    seat_state: SeatState,
    fractional_state: crate::fractional_scale::FractionalScaleState,
    viewporter: crate::viewporter::ViewporterState,
    shm_state: Shm,
    layer_shell: LayerShell,
}

pub struct ShareableState {
    wayland: WaylandState,
    data: Data,

    wgpu: renderer::State,
    app_pipeline: canvas::pipeline::DrawPipeline,
    loop_handle: LoopHandle<'static, State>,
}

pub struct Data {
    pub mode: OverlayMode,
    pub first_surface: Option<WlSurface>,
    // pub toolbar: crate::iced::IcedProgram<Toolbar>,
}

/// Indicates the mode that the overlay is currently in.
#[derive(Copy, Clone, Debug)]
// FIXME: deal with unused
#[allow(unused)]
pub enum OverlayMode {
    /// This occurs when the overlay has been toggled on (or held) by a non-tool
    /// specific keybind, all tools are accessible in this mode.
    Interactive,
    /// This occurs when the overlay has been toggled on (or held) by a tool
    /// specific keybind, other tools are non-accessible in this mode.
    Keybind,
    /// This occurs when the overlay is visible but non-interactive.
    Visible,
    /// This occurs when the user has hidden the overlay.
    Hidden,
}

impl WaylandConnection {
    #[instrument(name = "WaylandConnection::new")]
    pub async fn new() -> Result<WaylandConnection> {
        let event_loop = EventLoop::try_new().expect("failed to create event loop");

        trace!("connecting to wayland server");
        // Connect to the wayland compositor (server) using the socket at the standard location.
        let connection = Connection::connect_to_env()
            .expect("this program requires a running wayland compositor");
        // Enumerate the list of globals to get the protocols the server implements.
        trace!("fetching globals and event queue");
        let (globals, event_queue) = registry_queue_init(&connection)
            .expect("wayland: failed to initialise event queue and retrieve globals");
        let queue_handle = event_queue.handle();
        trace!("binding to registry state");
        let registry_state = RegistryState::new(&globals);

        // The compositor (not to be confused with the server which is commonly called the compositor) allows
        // configuring surfaces to be presented.
        trace!("binding to the compositor state");
        let compositor = CompositorState::bind(&globals, &queue_handle)
            .context("wl_compositor is not available")?;

        trace!("binding to seat state");
        let seat_state = SeatState::new(&globals, &queue_handle);
        trace!("binding to output state");
        let output_state = OutputState::new(&globals, &queue_handle);

        trace!("binding to layer shell");
        // This app uses the wlr layer shell, which may not be available with every compositor.
        let layer_shell =
            LayerShell::bind(&globals, &queue_handle).context("layer shell is not available")?;

        let fractional_state =
            crate::fractional_scale::FractionalScaleState::bind(&globals, &queue_handle)
                .context("fractional scale manager is not available")?;

        let viewporter = crate::viewporter::ViewporterState::bind(&globals, &queue_handle)
            .context("viewporter is not available")?;

        trace!("binding to shm");
        let shm_state = Shm::bind(&globals, &queue_handle).context("shm is not available")?;

        let wgpu = renderer::State::init().await?;

        let wayland = WaylandState {
            connection,
            compositor,
            output_state,
            queue_handle,
            registry_state,
            seat_state,
            shm_state,
            fractional_state,
            viewporter,
            layer_shell,
        };
        let data = Data {
            mode: OverlayMode::Hidden,
            first_surface: None,
        };

        let app_pipeline = canvas::pipeline::DrawPipeline::new(&wgpu.device, wgpu.texture_format);

        let shareable = ShareableState {
            wayland,
            data,
            wgpu,
            app_pipeline,
            loop_handle: event_loop.handle(),
        };

        let views = ViewManager {
            layer_shell_views: Default::default(),
        };

        let state = State {
            shareable,

            pointers: HashMap::new(),
            keyboards: HashMap::new(),

            views,
        };

        let mut wayland_connection = WaylandConnection {
            event_loop,
            event_queue,
            state,
        };

        // We request new information from the compositor to get information
        // about initial state such as outputs.
        trace!("Requesting updated state from the compositor");
        wayland_connection.round_trip()?;

        Ok(wayland_connection)
    }

    #[instrument(name = "WaylandConnection::round_trip", skip_all)]
    fn round_trip(&mut self) -> Result<usize> {
        self.event_queue
            .roundtrip(&mut self.state)
            .context("event queue failed to do a round trip")
    }

    #[instrument(name = "WaylandConnection::run", skip(self))]
    pub async fn run(mut self) -> Result<()> {
        let surface = self
            .state
            .shareable
            .wayland
            .compositor
            .create_surface(&self.state.shareable.wayland.queue_handle);
        surface.commit();

        // FIXME: deal with this not providing error messages since the wgpu and wayland stuff is
        // not cleaned up before it returns.
        let keybind_source = WaylandKeybinds::build_source().await;

        let ws = WaylandSource::new(
            self.state.shareable.wayland.connection.clone(),
            self.event_queue,
        );
        ws.insert(self.event_loop.handle())
            .expect("Failed to insert wayland event source into event loop");

        match keybind_source {
            Ok(source) => {
                self.event_loop
                    .handle()
                    .insert_source(source, Self::handle_global_keybind)
                    .expect("failed to insert global keybind event source into event loop");
            }
            Err(e) => {
                warn!("failed to bind global keybinds: {e:?}");
            }
        }

        self.event_loop
            .run(None, &mut self.state, |_| {})
            .context("failed to run event loop")
    }
    #[instrument(name = "WaylandConnection::get_displays", skip(self))]
    fn get_displays(&self) -> Result<(u32, u32, u32)> {
        self.state
            .shareable
            .wayland
            .output_state
            .outputs()
            .find_map(|output| {
                trace!("Checking output {:?}", output);
                if let Some(info) = self.state.shareable.wayland.output_state.info(&output) {
                    trace!("Found info: output modes");
                    let current_mode = info.modes.iter().find(|mode| {
                        trace!("{mode:?}");
                        mode.current
                    })?;
                    return Some((
                        TryInto::<u32>::try_into(current_mode.dimensions.0)
                            .context("display width must be positive")
                            .ok()?,
                        TryInto::<u32>::try_into(current_mode.dimensions.1)
                            .context("display height must be positive")
                            .ok()?,
                        TryInto::<u32>::try_into(info.scale_factor)
                            .context("scale factor must be positive")
                            .ok()?,
                    ));
                } else {
                    trace!("This output has no info attached!");
                };
                None
            })
            .context("failed to find the current monitor")
    }

    #[instrument(name = "WaylandConnection::outputs", skip_all)]
    pub fn outputs(&self) -> impl Iterator<Item = WlOutput> {
        self.state.shareable.wayland.output_state.outputs()
    }

    #[instrument(name = "WaylandConnection::handle_global_keybind", skip_all)]
    pub fn handle_global_keybind(
        event: calloop::channel::Event<(ShortcutEvents, Shortcuts)>,
        _: &mut (),
        state: &mut State,
    ) {
        let calloop::channel::Event::Msg(ev) = event else {
            panic!("global keybind source closed.");
        };
        trace!("received global keybind event {ev:?}");
        state.shareable.data.first_surface = None;
        match ev {
            (ShortcutEvents::Pressed, Shortcuts::Toggle) => {
                let mode = match state.shareable.data.mode {
                    OverlayMode::Interactive | OverlayMode::Keybind => OverlayMode::Visible,
                    OverlayMode::Visible | OverlayMode::Hidden => OverlayMode::Interactive,
                };
                state.shareable.data.mode = mode;
                for view in &mut state.views.canvas_views() {
                    view.set_mode(&mut state.shareable, mode)
                        .expect("failed to change mode");
                }
            }
            _ => {}
        }
        info!("got global keybind {ev:?}")
    }
}

impl CompositorHandler for State {
    #[instrument(name = "WaylandState::scale_factor_changed", skip_all)]
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wayland_client::protocol::wl_surface::WlSurface,
        _new_factor: i32,
    ) {
        info!("`scale_factor_changed` called");
    }

    #[instrument(name = "WaylandState::transform_changed", skip_all)]
    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wayland_client::protocol::wl_surface::WlSurface,
        _new_transform: wayland_client::protocol::wl_output::Transform,
    ) {
        info!("`transform_changed` called");
    }

    #[instrument(name = "WaylandState::frame", skip_all)]
    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wayland_client::protocol::wl_surface::WlSurface,
        _time: u32,
    ) {
        info!("`frame` called");
    }

    #[instrument(name = "WaylandState::surface_enter", skip_all)]
    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wayland_client::protocol::wl_surface::WlSurface,
        _output: &WlOutput,
    ) {
        info!("`surface_enter` called");
    }

    #[instrument(name = "WaylandState::surface_leave", skip_all)]
    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wayland_client::protocol::wl_surface::WlSurface,
        _output: &WlOutput,
    ) {
        info!("`surface_leave` called");
    }
}
impl OutputHandler for State {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.shareable.wayland.output_state
    }

    fn new_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: WlOutput) {
        info!("`new_output` called");
        // TODO: create overlay
        // TODO: check if monitor is in disallow config.
    }

    fn update_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: WlOutput) {
        info!("`update_output` called");
        // TODO: work out scaling for canvas
    }

    fn output_destroyed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: WlOutput) {
        // TODO: handle cleanup of views + layershell
        info!("`output_destroyed` called");
    }
}

impl LayerShellHandler for State {
    #[instrument(name = "WaylandState::closed", skip_all)]
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, layer: &LayerSurface) {
        // todo handle cleanup of surface assets.
        warn!("layer has been closed");
        // FIXME: use viewmanager::from_surface
        if let Some(view_idx) = self
            .views
            .layer_shell_views
            .iter_mut()
            .position(|v| &v.layer_surface == layer)
        {
            self.views.layer_shell_views.remove(view_idx);
        }
    }

    #[instrument(name = "LayerShellHandler::configure", skip_all)]
    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        info!(
            "layer shell handler called configure with {}x{}!",
            configure.new_size.0, configure.new_size.1
        );

        let view = self
            .views
            .layer_shell_views
            .iter_mut()
            .find(|v| &v.layer_surface == layer)
            .expect("`configure` called for a layer shell view not in the manager");

        view.configure(
            &mut self.shareable,
            configure.new_size.0,
            configure.new_size.1,
        );
    }
}

impl ShmHandler for State {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shareable.wayland.shm_state
    }
}

impl FractionalScaleHandler for State {
    fn preferred_scale(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        surface: &WlSurface,
        scale: u32,
    ) {
        self.views.from_surface(surface).set_scale_factor(
            &mut self.shareable,
            <u32 as TryInto<f64>>::try_into(scale)
                .expect("fractional scale factor doesn't fit in f64")
                / 120.,
        );
    }
}

#[derive(Clone)]
pub struct RedrawManager {
    loop_handle: LoopHandle<'static, State>,
    time_per_frame: Option<std::time::Duration>,
    animation_time_per_frame: std::time::Duration,

    last_redraw: Rc<Cell<Instant>>,
    pending_redraw: Rc<Cell<bool>>,
    animation_end: Rc<Cell<Option<Instant>>>,
}

impl RedrawManager {
    pub fn new(
        loop_handle: LoopHandle<'static, State>,
        fps: Option<u64>,
        animation_fps: u64,
    ) -> Self {
        Self {
            loop_handle,
            time_per_frame: fps.map(|x| std::time::Duration::from_millis(1000 / x)),
            animation_time_per_frame: std::time::Duration::from_millis(1000 / animation_fps),

            pending_redraw: Rc::new(Cell::new(false)),
            last_redraw: Rc::new(Cell::new(Instant::now())),
            animation_end: Rc::new(Cell::new(None)),
        }
    }

    pub fn insert(&self, timer: Timer) {
        let last_redraw = self.last_redraw.clone();
        let pending_redraw = self.pending_redraw.clone();
        let animation_end = self.animation_end.clone();

        let animation_frame_duration = self.animation_time_per_frame;

        self.loop_handle
            .insert_source(timer, move |_, _, state| {
                let now = Instant::now();
                last_redraw.set(now);

                for canvas in state.views.canvas_views() {
                    _ = canvas.render(&mut state.shareable);
                }

                let is_animating = match animation_end.get() {
                    Some(end_time) => {
                        match Instant::now().checked_duration_since(end_time) {
                            Some(remaining) => remaining < animation_frame_duration,
                            None => true, // end_time is in the future, so still animating
                        }
                    }
                    None => false, // no animation end time set
                };

                if is_animating {
                    TimeoutAction::ToInstant(now + animation_frame_duration)
                } else {
                    animation_end.set(None);
                    pending_redraw.set(false);
                    TimeoutAction::Drop
                }
            })
            .expect("Failed to insert timer into event loop");
    }
}
impl RedrawRequest for RedrawManager {
    fn request_redraw(&self) {
        tracing::trace!("Redraw requested!");
        if !self.pending_redraw.get() {
            self.pending_redraw.set(true);
            let needed_time =
                self.last_redraw.get() + self.time_per_frame.unwrap_or(Duration::ZERO);
            if needed_time <= Instant::now() {
                self.insert(Timer::immediate());
            } else {
                self.insert(Timer::from_deadline(needed_time));
            }
        } else {
            tracing::trace!("redraw already in progress");
        }
    }

    fn request_redraw_duration(&self, duration: Duration) {
        tracing::trace!("Redraw requested with duration: {duration:?}!");
        self.pending_redraw.set(true);
        if let Some(animation_end) = self.animation_end.get() {
            self.animation_end
                .set(Some((Instant::now() + duration).max(animation_end)));
        } else {
            self.animation_end.set(Some(Instant::now() + duration));
        }
        self.insert(Timer::immediate());
    }
}

delegate_compositor!(State);
delegate_output!(State);
delegate_seat!(State);
delegate_shm!(State);

delegate_keyboard!(State);
delegate_pointer!(State);

delegate_layer!(State);
crate::delegate_fractional_scale!(State);
crate::delegate_viewporter!(State);

delegate_registry!(State);

impl ProvidesRegistryState for State {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.shareable.wayland.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

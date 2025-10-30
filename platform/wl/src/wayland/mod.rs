use std::{
    collections::HashMap,
    i32,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use calloop::{
    ping::{make_ping, Ping},
    timer::TimeoutAction,
    EventLoop,
    LoopHandle,
};
use calloop_wayland_source::WaylandSource;
use color_eyre::eyre::{Context, OptionExt, Result};
use euclid::default::Size2D;
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor,
    delegate_keyboard,
    delegate_layer,
    delegate_output,
    delegate_pointer,
    delegate_registry,
    delegate_seat,
    delegate_shm,
    output::{OutputHandler, OutputInfo, OutputState},
    reexports::client::{
        backend::ObjectId,
        globals::registry_queue_init,
        protocol::{
            wl_output::{self, WlOutput},
            wl_surface::WlSurface,
        },
        Connection,
        EventQueue,
        QueueHandle,
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{pointer::ThemedPointer, SeatState},
    shell::{
        wlr_layer::{LayerShell, LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
        WaylandSurface,
    },
    shm::{Shm, ShmHandler},
};
use tracing::{info, instrument, trace, warn};

mod canvas_layer;
mod keyboard;
mod pointer;
pub(crate) mod protocols;
mod seat;

use canvas::{AnimationHandle, AnimationManager, RedrawRequest, RedrawRequestV2};

use crate::wayland::{
    canvas_layer::LayerShellCanvasView,
    keyboard::Keyboard,
    protocols::{FractionalScaleHandler, FractionalScaleState, ViewporterState},
};

// TODO: use is_wayland() function to detect.
#[allow(unused)]
pub fn is_wayland() -> bool {
    std::env::var("XDG_SESSION_TYPE").map_or_else(
        |_| std::env::var("WAYLAND_DISPLAY").is_ok(),
        |env| env == "wayland",
    )
}

pub struct WaylandConnection {
    event_queue: EventQueue<State>,
    pub event_loop: EventLoop<'static, State>,
    pub state: State,
}

pub struct State {
    pub shareable: ShareableState,

    pointers: HashMap<ObjectId, (ObjectId, ThemedPointer)>,
    keyboards: HashMap<ObjectId, Keyboard>,

    pub canvas_outputs: HashMap<WlOutput, LayerShellCanvasView>,
}

pub struct WaylandState {
    connection: Connection,
    compositor: CompositorState,
    output_state: OutputState,
    queue_handle: QueueHandle<State>,
    registry_state: RegistryState,
    seat_state: SeatState,
    fractional_state: protocols::FractionalScaleState,
    viewporter: protocols::ViewporterState,
    shm_state: Shm,
    layer_shell: LayerShell,
}

pub struct ShareableState {
    pub wayland: WaylandState,
    pub data: Data,

    wgpu: renderer::State,
    app_pipeline: canvas::pipeline::DrawPipeline,
    pub loop_handle: LoopHandle<'static, State>,
    pub redraw_manager: RedrawManager,
    pub redraw_manager_v2: RedrawManagerV2,
}

pub struct Data {
    pub first_surface: Option<WlSurface>,
    // pub toolbar: crate::iced::IcedProgram<Toolbar>,
}

/// Indicates the mode that the overlay is currently in.
#[derive(Clone, Copy, Debug)]
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
            .wrap_err("wl_compositor is not available")?;

        trace!("binding to seat state");
        let seat_state = SeatState::new(&globals, &queue_handle);
        trace!("binding to output state");
        let output_state = OutputState::new(&globals, &queue_handle);

        trace!("binding to layer shell");
        // This app uses the wlr layer shell, which may not be available with every compositor.
        let layer_shell =
            LayerShell::bind(&globals, &queue_handle).wrap_err("layer shell is not available")?;

        let fractional_state = FractionalScaleState::bind(&globals, &queue_handle)
            .wrap_err("fractional scale manager is not available")?;

        let viewporter = ViewporterState::bind(&globals, &queue_handle)
            .wrap_err("viewporter is not available")?;

        trace!("binding to shm");
        let shm_state = Shm::bind(&globals, &queue_handle).wrap_err("shm is not available")?;

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
            first_surface: None,
        };

        let app_pipeline = canvas::pipeline::DrawPipeline::new(&wgpu.device, wgpu.texture_format);

        let shareable = ShareableState {
            wayland,
            data,
            wgpu,
            app_pipeline,
            loop_handle: event_loop.handle(),
            redraw_manager: RedrawManager::new(event_loop.handle(), 60),
            redraw_manager_v2: RedrawManagerV2::new(event_loop.handle()),
        };

        let state = State {
            shareable,

            pointers: HashMap::new(),
            keyboards: HashMap::new(),

            canvas_outputs: HashMap::new(),
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

    fn round_trip(&mut self) -> Result<usize> {
        self.event_queue
            .roundtrip(&mut self.state)
            .wrap_err("event queue failed to do a round trip")
    }

    pub fn run(mut self) -> Result<()> {
        let surface = self
            .state
            .shareable
            .wayland
            .compositor
            .create_surface(&self.state.shareable.wayland.queue_handle);
        surface.commit();

        // FIXME: deal with this not providing error messages since the wgpu and wayland stuff is
        // not cleaned up before it returns.
        // let keybind_source = WaylandKeybinds::build_source().await;

        let ws = WaylandSource::new(
            self.state.shareable.wayland.connection.clone(),
            self.event_queue,
        );
        ws.insert(self.event_loop.handle())
            .expect("Failed to insert wayland event source into event loop");

        // match keybind_source {
        //     Ok(source) => {
        //         self.event_loop
        //             .handle()
        //             .insert_source(source, Self::handle_global_keybind)
        //             .expect("failed to insert global keybind event source into event loop");
        //     }
        //     Err(e) => {
        //         warn!("failed to bind global keybinds: {e:?}");
        //     }
        // }

        self.event_loop
            .run(None, &mut self.state, |_| {})
            .wrap_err("failed to run event loop")
    }

    pub fn wayland(&self) -> &WaylandState {
        &self.state.shareable.wayland
    }

    // pub fn handle_global_keybind(
    //     event: calloop::channel::Event<(ShortcutEvents, Shortcuts)>,
    //     _: &mut (),
    //     state: &mut State,
    // ) {
    //     let calloop::channel::Event::Msg(ev) = event else {
    //         panic!("global keybind source closed.");
    //     };
    //     trace!("received global keybind event {ev:?}");
    //     state.shareable.data.first_surface = None;
    //     match ev {
    //         (ShortcutEvents::Pressed, Shortcuts::Toggle) => {
    //             let mode = match state.shareable.data.mode {
    //                 OverlayMode::Interactive | OverlayMode::Keybind => OverlayMode::Visible,
    //                 OverlayMode::Visible | OverlayMode::Hidden => OverlayMode::Interactive,
    //             };
    //             state.shareable.data.mode = mode;
    //             for view in &mut state.views.canvas_views() {
    //                 view.set_mode(&mut state.shareable, mode).expect("failed to change mode");
    //             }
    //         }
    //         _ => {}
    //     }
    //     info!("got global keybind {ev:?}")
    // }
}
impl State {
    pub fn add_canvas_output(&mut self, output: WlOutput) {
        self.canvas_outputs
            .entry(output.clone())
            .or_insert_with(|| {
                LayerShellCanvasView::new(&mut self.shareable, &output, OverlayMode::Interactive)
                    .expect("failed to build layer shell canvas view")
            });
    }

    pub fn from_surface<'a>(
        outputs: &'a mut HashMap<WlOutput, LayerShellCanvasView>,
        surface: &WlSurface,
    ) -> Option<&'a mut LayerShellCanvasView> {
        outputs.values_mut().find(|x| x.surface() == surface)
    }
}

impl WaylandState {
    pub fn get_displays(&self) -> Vec<(WlOutput, OutputInfo)> {
        let output_state = &self.output_state;
        output_state
            .outputs()
            .filter_map(
                |output| match Self::extract_display_info(output_state, &output) {
                    Ok((info, _)) => Some((output, info)),
                    Err(err) => {
                        tracing::warn!("{err}");
                        None
                    }
                },
            )
            .collect::<Vec<_>>()
    }
    fn extract_display_info(
        output_state: &OutputState,
        output: &WlOutput,
    ) -> Result<(OutputInfo, Size2D<i32>)> {
        let info = output_state
            .info(output)
            .ok_or_eyre(format!("failed to get output info for output: {output:?}"))?;

        let current_mode = info
            .modes
            .iter()
            .find(|mode| {
                trace!("{mode:?}");
                mode.current
            })
            .ok_or_eyre(format!(
                "failed to find the current mode for the following output: {info:?}"
            ))?;

        let size = Size2D::new(current_mode.dimensions.0, current_mode.dimensions.1);

        Ok((info, size))
    }
}

impl CompositorHandler for State {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _new_factor: i32,
    ) {
        // We don't care about this since we are using fractional scaling.
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _new_transform: wl_output::Transform,
    ) {
        // We don't care about this since we currently don't pre-apply transform
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        surface: &WlSurface,
        _time: u32,
    ) {
        if let Some(view) = Self::from_surface(&mut self.canvas_outputs, surface) {
            if let Err(e) = view.render(&mut self.shareable) {
                tracing::error!("error while rendering canvas view: {e:?}");
            }
            if self
                .shareable
                .redraw_manager_v2
                .animation_manager
                .is_animating()
            {
                self.shareable.redraw_manager_v2.request_redraw();
            }
        } else {
            warn!("`frame` called for surface not in canvas_outputs");
        }
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _output: &WlOutput,
    ) {
        info!("`surface_enter` called");
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _output: &WlOutput,
    ) {
        info!("`surface_leave` called");
    }
}

// TODO: rewrite this into a client-server architecture.
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
        // TODO: work out scaling for canvas
        info!("`update_output` called");
    }

    fn output_destroyed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, output: WlOutput) {
        // TODO: handle cleanup of views + layershell, create backup file.
        info!("`output_destroyed` called");
        self.canvas_outputs.remove(&output);
    }
}

impl LayerShellHandler for State {
    #[instrument(name = "WaylandState::closed", skip_all)]
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        // todo handle cleanup of surface assets.
        warn!("layer has been closed");
        // FIXME: use viewmanager::from_surface
        // if let Some(view_idx) = self.views.layer_shell_views.iter_mut().position(|v| &v.layer_surface == layer) {
        //     let view = self.views.layer_shell_views.remove(view_idx);
        //     std::mem::drop(view.wgpu_surface);
        // }
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
            .canvas_outputs
            .values_mut()
            .find(|x| x.layer_surface == *layer);
        match view {
            Some(shell) => shell.configure(
                &mut self.shareable,
                configure.new_size.0,
                configure.new_size.1,
            ),
            None => tracing::warn!("Got configure event for non existent layer shell."),
        }

        // let view = self
        //     .views
        //     .layer_shell_views
        //     .iter_mut()
        //     .find(|v| &v.layer_surface == layer)
        //     .expect("`configure` called for a layer shell view not in the manager");
        //
        // view.configure(&mut self.shareable, configure.new_size.0, configure.new_size.1);
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
        if let Some(view) = Self::from_surface(&mut self.canvas_outputs, surface) {
            view.set_scale_factor(
                &mut self.shareable,
                <u32 as Into<f64>>::into(scale)
                    / 120.,
            );
        }
        // self.views.from_surface(surface).set_scale_factor(
        //     &mut self.shareable,
        //     <u32 as TryInto<f64>>::try_into(scale).expect("fractional scale factor doesn't fit in f64") / 120.,
        // );
    }
}

// Commands that can be sent to the event loop
enum RedrawCommand {
    Redraw,
    RedrawWithDuration(Duration),
}

#[derive(Clone)]
pub struct RedrawManager {
    sender: calloop::channel::Sender<RedrawCommand>,

    anim_frame_duration: Duration,
    current_anim_end: Arc<Mutex<Option<Instant>>>,
}

impl RedrawManager {
    pub fn new(handle: LoopHandle<'static, State>, animation_fps: u64) -> RedrawManager {
        let (sender, receiver) = calloop::channel::channel();

        let manager = Self {
            sender,

            anim_frame_duration: Duration::from_millis(1000 / animation_fps),
            current_anim_end: Arc::new(Mutex::new(None)),
        };

        let animation_end = manager.current_anim_end.clone();
        let animation_frame_duration = manager.anim_frame_duration;

        let handle_clone = handle.clone();

        handle
            .insert_source(
                receiver,
                move |event: calloop::channel::Event<RedrawCommand>, _, state| {
                    match event {
                        calloop::channel::Event::Msg(cmd) => match cmd {
                            RedrawCommand::Redraw => {
                                tracing::trace!("Redraw requested!");

                                do_redraw(state);
                            }
                            RedrawCommand::RedrawWithDuration(duration) => {
                                tracing::trace!("Redraw requested with duration: {:?}", duration);
                                let mut anim_end = animation_end.lock().unwrap();
                                let now = Instant::now();
                                *anim_end = Some(
                                    anim_end
                                        .map(|old| (now + duration).max(old))
                                        .unwrap_or(now + duration),
                                );
                                do_redraw(state);
                                // schedule repeated frames until animation_end passes
                                let animation_end = animation_end.clone();
                                handle_clone
                                    .insert_source(
                                        calloop::timer::Timer::from_duration(
                                            animation_frame_duration,
                                        ),
                                        move |_, _, state| {
                                            let now = Instant::now();
                                            let end = *animation_end.lock().unwrap();
                                            let is_animating = end.is_some_and(|e| now < e);
                                            if is_animating {
                                                do_redraw(state);
                                                TimeoutAction::ToDuration(animation_frame_duration)
                                            } else {
                                                *animation_end.lock().unwrap() = None;
                                                TimeoutAction::Drop
                                            }
                                        },
                                    )
                                    .unwrap();
                            }
                        },
                        calloop::channel::Event::Closed => {
                            panic!("redraw channel closed");
                        }
                    }
                },
            )
            .unwrap();

        manager
    }
}

impl RedrawRequest for RedrawManager {
    fn request_redraw(&self) {
        let _ = self.sender.send(RedrawCommand::Redraw);
    }

    fn request_redraw_duration(&self, duration: Duration) {
        let _ = self
            .sender
            .send(RedrawCommand::RedrawWithDuration(duration));
    }
}

#[derive(Clone)]
pub struct RedrawManagerV2 {
    animation_manager: Arc<AnimationManager>,
    redraw_ping: Ping,
}
impl RedrawManagerV2 {
    pub fn new(loop_handle: LoopHandle<'static, State>) -> Self {
        let animation_manager = Arc::new(AnimationManager::new());
        let (ping, source) = make_ping().expect("failed to create redraw ping source");

        loop_handle
            .insert_source(source, {
                // let ping = ping.clone();
                // let animation_manager = animation_manager.clone();
                move |_, _, state| {
                    do_redraw(state);
                    // if animation_manager.is_animating() {
                    //     ping.ping();
                    // }
                }
            })
            .expect("failed to insert redraw ping source");
        Self {
            animation_manager,
            redraw_ping: ping,
        }
    }
}

impl RedrawRequestV2 for RedrawManagerV2 {
    fn request_redraw(&self) {
        self.redraw_ping.ping();
    }

    fn new_animation_handle(&self) -> AnimationHandle {
        self.animation_manager.new_handle()
    }
}

fn do_redraw(state: &mut State) {
    for canvas in state.canvas_outputs.values() {
        canvas.layer_surface.wl_surface().frame(
            &state.shareable.wayland.queue_handle,
            canvas.layer_surface.wl_surface().clone(),
        );
        canvas.layer_surface.wl_surface().commit();
    }
}

delegate_compositor!(State);
delegate_output!(State);
delegate_seat!(State);
delegate_shm!(State);

delegate_keyboard!(State);
delegate_pointer!(State);

delegate_layer!(State);
protocols::delegate_fractional_scale!(State);
protocols::delegate_viewporter!(State);

delegate_registry!(State);

impl ProvidesRegistryState for State {
    registry_handlers![OutputState, SeatState];

    fn registry(&mut self) -> &mut RegistryState {
        &mut self.shareable.wayland.registry_state
    }
}

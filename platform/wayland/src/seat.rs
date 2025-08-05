use input::sctk::KeyEventKind;
use smithay_client_toolkit::seat::{
    keyboard::repeat::RepeatCallback, pointer::ThemeSpec, Capability, SeatHandler, SeatState,
};
use tracing::{instrument, trace, warn};
use wayland_client::{protocol::wl_seat::WlSeat, Connection, Proxy, QueueHandle};

use crate::keyboard::Keyboard;
use crate::State;

impl SeatHandler for State {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.shareable.wayland.seat_state
    }

    fn new_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: WlSeat) {
        trace!("Adding new seat...")
    }
    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, seat: WlSeat) {
        trace!("Removing seat...");
        // remove the pointer and keyboard if they haven't been already
        self.pointers.remove(&seat.id());
        self.keyboards.remove(&seat.id());
    }

    #[instrument(name = "SeatHandler::new_capability", skip_all)]
    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: WlSeat,
        capability: Capability,
    ) {
        match capability {
            Capability::Keyboard => {
                // FIXME: dedup
                let callback: RepeatCallback<State> = Box::new(move |state, keyboard, event| {
                    let Some(kb) = state.keyboards.values().find(|x| x.id() == keyboard.id())
                    else {
                        warn!("keyboard event `release_key` dispatched for keyboard not in state");
                        return;
                    };
                    let Some(surface) = &kb.last_surface else {
                        return;
                    };
                    let view = state.views.from_surface(&surface);

                    view.keyboard_event(
                        &mut state.shareable,
                        &KeyEventKind::Press((event.clone(), kb.last_modifiers)),
                    );
                    view.keyboard_event(
                        &mut state.shareable,
                        &KeyEventKind::Release((event, kb.last_modifiers)),
                    );
                });

                trace!("Adding keyboard capability");
                let keyboard = self
                    .shareable
                    .wayland
                    .seat_state
                    .get_keyboard_with_repeat(
                        qh,
                        &seat,
                        None,
                        self.shareable.loop_handle.clone(),
                        callback,
                    )
                    .expect("Failed to create keyboard");
                self.keyboards.insert(seat.id(), Keyboard::new(keyboard));
            }

            Capability::Pointer => {
                trace!("Adding pointer capability");
                let cursor_surface = self.shareable.wayland.compositor.create_surface(qh);
                let themed_pointer = self
                    .shareable
                    .wayland
                    .seat_state
                    .get_pointer_with_theme(
                        qh,
                        &seat,
                        self.shareable.wayland.shm_state.wl_shm(),
                        cursor_surface,
                        ThemeSpec::System,
                    )
                    .expect("Failed to create themed pointer");
                self.pointers
                    .insert(seat.id(), (themed_pointer.pointer().id(), themed_pointer));
            }
            _ => {}
        }
    }

    #[instrument(name = "SeatHandler::remove_capability", skip_all)]
    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        seat: WlSeat,
        capability: Capability,
    ) {
        trace!("capability removed!");
        match capability {
            Capability::Keyboard => {
                self.keyboards.remove(&seat.id());
            }

            Capability::Pointer => {
                self.pointers.remove(&seat.id());
            }
            _ => {}
        };
    }
}

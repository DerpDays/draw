use smithay_client_toolkit::{
    reexports::client::{Connection, Proxy, QueueHandle, protocol::wl_pointer},
    seat::pointer::{PointerEvent, PointerHandler},
};

use crate::wayland::State;

impl PointerHandler for State {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            _ = self
                .shareable
                .data
                .first_surface
                .get_or_insert(event.surface.clone());

            // let view = self.views.from_surface(&event.surface);
            let view = self
                .canvas_outputs
                .values_mut()
                .find(|x| *x.surface() == event.surface);

            if let Some((_, themed_pointer)) =
                self.pointers.values_mut().find(|x| x.0 == pointer.id())
            {
                if let Some(view) = view {
                    view.pointer_event(&mut self.shareable, themed_pointer, event);
                } else {
                    tracing::warn!("pointer event sent to surface not in canvas_outputs");
                }
            } else {
                tracing::warn!("pointer event `pointer_frame` dispatched for pointer not in state");
            };
        }
    }
}

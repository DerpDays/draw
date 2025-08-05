use smithay_client_toolkit::seat::pointer::PointerHandler;
use wayland_client::{protocol::wl_pointer, Connection, Proxy, QueueHandle};

use crate::State;

impl PointerHandler for State {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        pointer: &wl_pointer::WlPointer,
        events: &[smithay_client_toolkit::seat::pointer::PointerEvent],
    ) {
        for event in events {
            _ = self
                .shareable
                .data
                .first_surface
                .get_or_insert(event.surface.clone());

            let view = self.views.from_surface(&event.surface);

            if let Some((_, themed_pointer)) =
                self.pointers.values_mut().find(|x| x.0 == pointer.id())
            {
                view.pointer_event(&mut self.shareable, &themed_pointer, event);
            } else {
                tracing::warn!(
                    "pointer event `pointer_frame` dispatched for keyboard not in state"
                );
                return;
            };
        }
    }
}

use input::sctk::KeyEventKind;
use smithay_client_toolkit::{
    reexports::client::{
        Connection, Proxy, QueueHandle,
        backend::ObjectId,
        protocol::{wl_keyboard::WlKeyboard, wl_surface::WlSurface},
    },
    seat::keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers},
};
use tracing::warn;

use crate::wayland::State;

#[derive(Clone)]
pub struct Keyboard {
    pub keyboard: WlKeyboard,
    pub last_surface: Option<WlSurface>,
    pub last_modifiers: Modifiers,
}

impl Keyboard {
    pub fn new(keyboard: WlKeyboard) -> Self {
        Self {
            keyboard,
            last_surface: None,
            last_modifiers: Modifiers::default(),
        }
    }

    pub fn id(&self) -> ObjectId {
        self.keyboard.id()
    }

    pub(crate) fn repeat_callback(state: &mut State, keyboard: &WlKeyboard, event: KeyEvent) {
        let Some(kb) = state.keyboards.values().find(|x| x.id() == keyboard.id()) else {
            warn!("keyboard event `release_key` dispatched for keyboard not in state");
            return;
        };
        let Some(surface) = &kb.last_surface else {
            return;
        };
        if let Some(view) = State::from_surface(&mut state.canvas_outputs, surface) {
            view.keyboard_event(
                &mut state.shareable,
                &KeyEventKind::Press((event.clone(), kb.last_modifiers)),
            );
            view.keyboard_event(
                &mut state.shareable,
                &KeyEventKind::Release((event, kb.last_modifiers)),
            );
        };
    }
}

impl KeyboardHandler for State {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        keyboard: &WlKeyboard,
        surface: &WlSurface,
        _serial: u32,
        _raw: &[u32],
        _keysyms: &[Keysym],
    ) {
        let Some(kb) = self
            .keyboards
            .values_mut()
            .find(|x| x.id() == keyboard.id())
        else {
            warn!("keyboard event `enter` dispatched for keyboard not in state");
            return;
        };
        kb.last_surface = Some(surface.clone());
    }

    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        keyboard: &WlKeyboard,
        _surface: &WlSurface,
        _serial: u32,
    ) {
        let Some(kb) = self
            .keyboards
            .values_mut()
            .find(|x| x.id() == keyboard.id())
        else {
            warn!("keyboard event `leave` dispatched for keyboard not in state");
            return;
        };
        kb.last_surface = None;
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        keyboard: &WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        let Some(kb) = self.keyboards.values().find(|x| x.id() == keyboard.id()) else {
            warn!("keyboard event `press_key` dispatched for keyboard not in state");
            return;
        };

        if event.keysym == Keysym::Escape && kb.last_modifiers.alt {
            for view in self.canvas_outputs.values_mut() {
                let _ = view.set_mode(&mut self.shareable, crate::wayland::OverlayMode::Hidden);
                if let Err(e) = view.render(&mut self.shareable) {
                    warn!("render failed for layershell view: {e:?}");
                }
            }
        };

        let Some(surface) = &kb.last_surface else {
            return;
        };

        if let Some(view) = State::from_surface(&mut self.canvas_outputs, surface) {
            view.keyboard_event(
                &mut self.shareable,
                &KeyEventKind::Press((event, kb.last_modifiers)),
            );
        };
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        keyboard: &WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        let Some(kb) = self.keyboards.values().find(|x| x.id() == keyboard.id()) else {
            warn!("keyboard event `release_key` dispatched for keyboard not in state");
            return;
        };
        let Some(surface) = &kb.last_surface else {
            return;
        };

        if let Some(view) = State::from_surface(&mut self.canvas_outputs, surface) {
            view.keyboard_event(
                &mut self.shareable,
                &KeyEventKind::Release((event, kb.last_modifiers)),
            );
        }
    }

    fn update_modifiers(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        keyboard: &WlKeyboard,
        _serial: u32,
        modifiers: Modifiers,
        _raw_modifiers: RawModifiers,
        _layout: u32,
    ) {
        let Some(kb) = self
            .keyboards
            .values_mut()
            .find(|x| x.id() == keyboard.id())
        else {
            warn!("keyboard event `release_key` dispatched for keyboard not in state");
            return;
        };
        kb.last_modifiers = modifiers;
        let Some(surface) = &kb.last_surface else {
            return;
        };

        if let Some(view) = State::from_surface(&mut self.canvas_outputs, surface) {
            view.keyboard_event(
                &mut self.shareable,
                &KeyEventKind::ModifiersChanged(modifiers),
            );
        }
    }
    fn repeat_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &smithay_client_toolkit::reexports::client::protocol::wl_keyboard::WlKeyboard,
        _serial: u32,
        _event: KeyEvent,
    ) {
        // TODO: wl_seat v10 as of 20-8-2025 has not been implemented into compositors yet.
        // We should support this instead of get_keyboard_with_repeat(..)
    }
}

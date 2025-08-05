use input::sctk::KeyEventKind;
use smithay_client_toolkit::seat::keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers};
use tracing::warn;
use wayland_backend::client::ObjectId;
use wayland_client::{
    protocol::{wl_keyboard::WlKeyboard, wl_surface::WlSurface},
    Connection, Proxy, QueueHandle,
};

use crate::views::{LayerShellView, View};
use crate::State;

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

        if event.keysym == Keysym::Escape {
            for view in &mut self.views.canvas_views() {
                let _ = view.set_mode(&mut self.shareable, crate::OverlayMode::Hidden);
                if let Err(e) = view.render(&mut self.shareable) {
                    warn!("render failed for layershell view: {e:?}");
                }
            }
        };

        let Some(surface) = &kb.last_surface else {
            return;
        };
        let view = self.views.from_surface(&surface);
        view.keyboard_event(
            &mut self.shareable,
            &KeyEventKind::Press((event, kb.last_modifiers)),
        );
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
        let view = self.views.from_surface(&surface);

        view.keyboard_event(
            &mut self.shareable,
            &KeyEventKind::Release((event, kb.last_modifiers)),
        );
    }

    fn update_modifiers(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        keyboard: &WlKeyboard,
        _serial: u32,
        modifiers: Modifiers,
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
        // dispatch the event
        let Some(surface) = &kb.last_surface else {
            return;
        };
        let view = self.views.from_surface(&surface);
        view.keyboard_event(
            &mut self.shareable,
            &KeyEventKind::ModifiersChanged(modifiers),
        );
    }
}

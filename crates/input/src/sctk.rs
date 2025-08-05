use smithay_client_toolkit::{
    reexports::client::protocol::wl_pointer,
    seat::{
        keyboard::{self, KeyEvent, Keysym},
        pointer::{self, PointerEventKind},
    },
};

#[derive(Clone, Debug)]
pub enum KeyEventKind {
    Press((KeyEvent, keyboard::Modifiers)),
    Release((KeyEvent, keyboard::Modifiers)),
    ModifiersChanged(keyboard::Modifiers),
}

use crate::{
    AxisScroll, AxisSource, CursorIcon, Key, KeyboardEvent, KeyboardEventKind, Modifiers,
    MouseButton, MouseEventKind, SpecialKey,
};

pub const fn pointer_event(kind: &PointerEventKind) -> MouseEventKind {
    match kind {
        PointerEventKind::Enter { .. } => MouseEventKind::Enter,
        PointerEventKind::Leave { .. } => MouseEventKind::Leave,
        PointerEventKind::Motion { time } => MouseEventKind::Motion { time: *time },
        PointerEventKind::Press { time, button, .. } => MouseEventKind::Press {
            time: *time,
            button: match *button {
                pointer::BTN_LEFT => MouseButton::Left,
                pointer::BTN_BACK => MouseButton::Back,
                pointer::BTN_SIDE => MouseButton::Side,
                pointer::BTN_TASK => MouseButton::Task,
                pointer::BTN_EXTRA => MouseButton::Extra,
                pointer::BTN_RIGHT => MouseButton::Right,
                pointer::BTN_MIDDLE => MouseButton::Middle,
                pointer::BTN_FORWARD => MouseButton::Forward,
                _ => MouseButton::Left,
            },
        },
        PointerEventKind::Release { time, button, .. } => MouseEventKind::Release {
            time: *time,
            button: match *button {
                pointer::BTN_LEFT => MouseButton::Left,
                pointer::BTN_BACK => MouseButton::Back,
                pointer::BTN_SIDE => MouseButton::Side,
                pointer::BTN_TASK => MouseButton::Task,
                pointer::BTN_EXTRA => MouseButton::Extra,
                pointer::BTN_RIGHT => MouseButton::Right,
                pointer::BTN_MIDDLE => MouseButton::Middle,
                pointer::BTN_FORWARD => MouseButton::Forward,
                _ => MouseButton::Left,
            },
        },
        PointerEventKind::Axis {
            time,
            horizontal,
            vertical,
            source,
        } => MouseEventKind::Axis {
            time: *time,
            horizontal: AxisScroll {
                absolute: horizontal.absolute,
                discrete: horizontal.discrete,
                stop: horizontal.stop,
            },
            vertical: AxisScroll {
                absolute: vertical.absolute,
                discrete: vertical.discrete,
                stop: vertical.stop,
            },
            source: match source {
                Some(wl_pointer::AxisSource::Wheel) => Some(AxisSource::Wheel),
                Some(wl_pointer::AxisSource::Finger) => Some(AxisSource::Finger),
                Some(wl_pointer::AxisSource::WheelTilt) => Some(AxisSource::WheelTilt),
                Some(wl_pointer::AxisSource::Continuous) => Some(AxisSource::Continuous),
                _ => None,
            },
        },
    }
}

pub fn keyboard_event(kind: &KeyEventKind) -> KeyboardEvent {
    match kind {
        KeyEventKind::Press((key, modifiers)) => KeyboardEvent {
            modifiers: self::modifiers(modifiers),
            kind: KeyboardEventKind::Press(self::key(&key)),
        },
        KeyEventKind::Release((key, modifiers)) => KeyboardEvent {
            modifiers: self::modifiers(modifiers),
            kind: KeyboardEventKind::Release(self::key(&key)),
        },
        KeyEventKind::ModifiersChanged(modifiers) => KeyboardEvent {
            modifiers: self::modifiers(modifiers),
            kind: KeyboardEventKind::ModifiersChanged,
        },
    }
}

pub const fn modifiers(modifiers: &keyboard::Modifiers) -> Modifiers {
    let mut new = Modifiers::empty();
    if modifiers.ctrl {
        new = new.union(Modifiers::CTRL);
    };
    if modifiers.alt {
        new = new.union(Modifiers::ALT);
    };
    if modifiers.shift {
        new = new.union(Modifiers::SHIFT);
    };
    if modifiers.logo {
        new = new.union(Modifiers::LOGO);
    };
    if modifiers.caps_lock {
        new = new.union(Modifiers::CAPS_LOCK);
    };
    if modifiers.num_lock {
        new = new.union(Modifiers::NUM_LOCK);
    };
    new
}

pub fn key(key: &KeyEvent) -> Key {
    let special_key = match key.keysym {
        Keysym::Shift_L => Key::SpecialKey(SpecialKey::LShift),
        Keysym::Shift_R => Key::SpecialKey(SpecialKey::RShift),
        Keysym::Control_L => Key::SpecialKey(SpecialKey::LCtrl),
        Keysym::Control_R => Key::SpecialKey(SpecialKey::RCtrl),

        Keysym::Super_L => Key::SpecialKey(SpecialKey::Logo),
        Keysym::Super_R => Key::SpecialKey(SpecialKey::Logo),

        Keysym::Alt_L => Key::SpecialKey(SpecialKey::RAlt),
        Keysym::Alt_R => Key::SpecialKey(SpecialKey::LAlt),

        Keysym::Caps_Lock => Key::SpecialKey(SpecialKey::CapsLock),
        Keysym::Num_Lock => Key::SpecialKey(SpecialKey::NumLock),

        Keysym::Escape => Key::SpecialKey(SpecialKey::Escape),
        Keysym::F1 => Key::SpecialKey(SpecialKey::F1),
        Keysym::F2 => Key::SpecialKey(SpecialKey::F2),
        Keysym::F3 => Key::SpecialKey(SpecialKey::F3),
        Keysym::F4 => Key::SpecialKey(SpecialKey::F4),
        Keysym::F5 => Key::SpecialKey(SpecialKey::F5),
        Keysym::F6 => Key::SpecialKey(SpecialKey::F6),
        Keysym::F7 => Key::SpecialKey(SpecialKey::F7),
        Keysym::F8 => Key::SpecialKey(SpecialKey::F8),
        Keysym::F9 => Key::SpecialKey(SpecialKey::F9),
        Keysym::F10 => Key::SpecialKey(SpecialKey::F10),
        Keysym::F11 => Key::SpecialKey(SpecialKey::F11),
        Keysym::F12 => Key::SpecialKey(SpecialKey::F12),

        Keysym::Insert => Key::SpecialKey(SpecialKey::Insert),
        Keysym::Print => Key::SpecialKey(SpecialKey::PrintScreen),
        Keysym::Delete => Key::SpecialKey(SpecialKey::Delete),

        Keysym::ISO_Enter => Key::SpecialKey(SpecialKey::Enter),
        Keysym::KP_Enter => Key::SpecialKey(SpecialKey::Enter),
        Keysym::Return => Key::SpecialKey(SpecialKey::Enter),
        Keysym::BackSpace => Key::SpecialKey(SpecialKey::Backspace),

        Keysym::Home => Key::SpecialKey(SpecialKey::Home),
        Keysym::End => Key::SpecialKey(SpecialKey::End),
        Keysym::Page_Up => Key::SpecialKey(SpecialKey::PageUp),
        Keysym::KP_Page_Up => Key::SpecialKey(SpecialKey::PageUp),
        Keysym::Page_Down => Key::SpecialKey(SpecialKey::PageDown),
        Keysym::KP_Page_Down => Key::SpecialKey(SpecialKey::PageDown),

        Keysym::Left => Key::SpecialKey(SpecialKey::Left),
        Keysym::Right => Key::SpecialKey(SpecialKey::Right),
        Keysym::Up => Key::SpecialKey(SpecialKey::Up),
        Keysym::Down => Key::SpecialKey(SpecialKey::Down),

        Keysym::Tab => Key::SpecialKey(SpecialKey::Tab),
        Keysym::KP_Tab => Key::SpecialKey(SpecialKey::Tab),

        _ => Key::Unknown,
    };
    match special_key {
        Key::Unknown => {
            if let Some(repr) = &key.utf8 {
                Key::Character(repr.clone())
            } else {
                Key::Unknown
            }
        }
        _ => special_key,
    }
}

pub const fn cursor_icon(icon: CursorIcon) -> pointer::CursorIcon {
    match icon {
        CursorIcon::Default => pointer::CursorIcon::Default,
        CursorIcon::ContextMenu => pointer::CursorIcon::ContextMenu,
        CursorIcon::Help => pointer::CursorIcon::Help,
        CursorIcon::Pointer => pointer::CursorIcon::Pointer,
        CursorIcon::Progress => pointer::CursorIcon::Progress,
        CursorIcon::Wait => pointer::CursorIcon::Wait,
        CursorIcon::Cell => pointer::CursorIcon::Cell,
        CursorIcon::Crosshair => pointer::CursorIcon::Crosshair,
        CursorIcon::Text => pointer::CursorIcon::Text,
        CursorIcon::VerticalText => pointer::CursorIcon::VerticalText,
        CursorIcon::Alias => pointer::CursorIcon::Alias,
        CursorIcon::Copy => pointer::CursorIcon::Copy,
        CursorIcon::Move => pointer::CursorIcon::Move,
        CursorIcon::NoDrop => pointer::CursorIcon::NoDrop,
        CursorIcon::NotAllowed => pointer::CursorIcon::NotAllowed,
        CursorIcon::Grab => pointer::CursorIcon::Grab,
        CursorIcon::Grabbing => pointer::CursorIcon::Grabbing,
        CursorIcon::EResize => pointer::CursorIcon::EResize,
        CursorIcon::NResize => pointer::CursorIcon::NResize,
        CursorIcon::NeResize => pointer::CursorIcon::NeResize,
        CursorIcon::NwResize => pointer::CursorIcon::NwResize,
        CursorIcon::SResize => pointer::CursorIcon::SResize,
        CursorIcon::SeResize => pointer::CursorIcon::SeResize,
        CursorIcon::SwResize => pointer::CursorIcon::SwResize,
        CursorIcon::WResize => pointer::CursorIcon::WResize,
        CursorIcon::EwResize => pointer::CursorIcon::EwResize,
        CursorIcon::NsResize => pointer::CursorIcon::NsResize,
        CursorIcon::NeswResize => pointer::CursorIcon::NeswResize,
        CursorIcon::NwseResize => pointer::CursorIcon::NwseResize,
        CursorIcon::ColResize => pointer::CursorIcon::ColResize,
        CursorIcon::RowResize => pointer::CursorIcon::RowResize,
        CursorIcon::AllScroll => pointer::CursorIcon::AllScroll,
        CursorIcon::ZoomIn => pointer::CursorIcon::ZoomIn,
        CursorIcon::ZoomOut => pointer::CursorIcon::ZoomOut,
    }
}

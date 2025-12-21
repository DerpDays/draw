use winit::{
    event::{self, ElementState, KeyEvent, MouseScrollDelta, TouchPhase},
    keyboard::{KeyCode, ModifiersState, PhysicalKey},
    window,
};

use crate::{
    AxisScroll,
    AxisSource,
    CursorIcon,
    Key,
    KeyboardEvent,
    KeyboardEventKind,
    Modifiers,
    MouseButton,
    MouseEventKind,
    SpecialKey,
};

#[derive(Clone, Debug)]
pub enum KeyEventKind {
    Press((KeyEvent, ModifiersState)),
    Release((KeyEvent, ModifiersState)),
    ModifiersChanged(ModifiersState),
}

#[derive(Clone, Debug)]
pub enum CursorEventKind {
    Enter,
    Leave,
    Motion,
    Input {
        state: ElementState,
        button: winit::event::MouseButton,
    },
    Axis {
        delta: MouseScrollDelta,
        phase: TouchPhase,
    },
}

pub fn pointer_event(kind: &CursorEventKind) -> MouseEventKind {
    match kind {
        CursorEventKind::Enter => MouseEventKind::Enter,
        CursorEventKind::Leave => MouseEventKind::Leave,
        CursorEventKind::Motion => MouseEventKind::Motion { time: 0 },
        CursorEventKind::Input { button, state } => {
            // MouseEventKind::Press {
            // time: 0,
            let button = match *button {
                event::MouseButton::Left => MouseButton::Left,
                event::MouseButton::Right => MouseButton::Right,
                event::MouseButton::Middle => MouseButton::Middle,
                event::MouseButton::Back => MouseButton::Back,
                event::MouseButton::Forward => MouseButton::Forward,
                // Numbers taken from https://docs.rs/smithay-client-toolkit/latest/src/smithay_client_toolkit/seat/pointer/mod.rs.html#39
                event::MouseButton::Other(307) => MouseButton::Side,
                event::MouseButton::Other(279) => MouseButton::Task,
                event::MouseButton::Other(276) => MouseButton::Extra,
                _ => MouseButton::Extra,
            };
            match state {
                ElementState::Pressed => MouseEventKind::Press { time: 0, button },
                ElementState::Released => MouseEventKind::Release { time: 0, button },
            }
        }
        CursorEventKind::Axis { delta, phase } => match delta {
            MouseScrollDelta::LineDelta(horizontal, vertical) => MouseEventKind::Axis {
                time: 0,
                horizontal: AxisScroll {
                    absolute: 0.,
                    discrete: *horizontal as i32,
                    stop: phase == &TouchPhase::Ended,
                },
                vertical: AxisScroll {
                    absolute: 0.,
                    discrete: *vertical as i32,
                    stop: phase == &TouchPhase::Ended,
                },
                source: Some(AxisSource::Wheel),
            },
            MouseScrollDelta::PixelDelta(position) => MouseEventKind::Axis {
                time: 0,
                horizontal: AxisScroll {
                    absolute: position.x,
                    discrete: 0,
                    stop: phase == &TouchPhase::Ended,
                },
                vertical: AxisScroll {
                    absolute: position.y,
                    discrete: 0,
                    stop: phase == &TouchPhase::Ended,
                },
                source: Some(AxisSource::Wheel),
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

pub const fn modifiers(modifiers: &ModifiersState) -> Modifiers {
    let mut new = Modifiers::empty();
    if modifiers.contains(ModifiersState::CONTROL) {
        new = new.union(Modifiers::CTRL);
    };
    if modifiers.contains(ModifiersState::ALT) {
        new = new.union(Modifiers::ALT);
    };
    if modifiers.contains(ModifiersState::SHIFT) {
        new = new.union(Modifiers::SHIFT);
    };
    if modifiers.contains(ModifiersState::SUPER) {
        new = new.union(Modifiers::LOGO);
    };
    new
}

pub fn key(key: &KeyEvent) -> Key {
    let special_key = match key.physical_key {
        PhysicalKey::Code(KeyCode::ShiftLeft) => Key::SpecialKey(SpecialKey::LShift),
        PhysicalKey::Code(KeyCode::ShiftRight) => Key::SpecialKey(SpecialKey::RShift),
        PhysicalKey::Code(KeyCode::ControlLeft) => Key::SpecialKey(SpecialKey::LCtrl),
        PhysicalKey::Code(KeyCode::ControlRight) => Key::SpecialKey(SpecialKey::RCtrl),

        PhysicalKey::Code(KeyCode::SuperLeft) => Key::SpecialKey(SpecialKey::Logo),
        PhysicalKey::Code(KeyCode::SuperRight) => Key::SpecialKey(SpecialKey::Logo),

        PhysicalKey::Code(KeyCode::AltLeft) => Key::SpecialKey(SpecialKey::RAlt),
        PhysicalKey::Code(KeyCode::AltRight) => Key::SpecialKey(SpecialKey::LAlt),

        PhysicalKey::Code(KeyCode::CapsLock) => Key::SpecialKey(SpecialKey::CapsLock),
        PhysicalKey::Code(KeyCode::NumLock) => Key::SpecialKey(SpecialKey::NumLock),

        PhysicalKey::Code(KeyCode::Escape) => Key::SpecialKey(SpecialKey::Escape),
        PhysicalKey::Code(KeyCode::F1) => Key::SpecialKey(SpecialKey::F1),
        PhysicalKey::Code(KeyCode::F2) => Key::SpecialKey(SpecialKey::F2),
        PhysicalKey::Code(KeyCode::F3) => Key::SpecialKey(SpecialKey::F3),
        PhysicalKey::Code(KeyCode::F4) => Key::SpecialKey(SpecialKey::F4),
        PhysicalKey::Code(KeyCode::F5) => Key::SpecialKey(SpecialKey::F5),
        PhysicalKey::Code(KeyCode::F6) => Key::SpecialKey(SpecialKey::F6),
        PhysicalKey::Code(KeyCode::F7) => Key::SpecialKey(SpecialKey::F7),
        PhysicalKey::Code(KeyCode::F8) => Key::SpecialKey(SpecialKey::F8),
        PhysicalKey::Code(KeyCode::F9) => Key::SpecialKey(SpecialKey::F9),
        PhysicalKey::Code(KeyCode::F10) => Key::SpecialKey(SpecialKey::F10),
        PhysicalKey::Code(KeyCode::F11) => Key::SpecialKey(SpecialKey::F11),
        PhysicalKey::Code(KeyCode::F12) => Key::SpecialKey(SpecialKey::F12),

        PhysicalKey::Code(KeyCode::Insert) => Key::SpecialKey(SpecialKey::Insert),
        PhysicalKey::Code(KeyCode::PrintScreen) => Key::SpecialKey(SpecialKey::PrintScreen),
        PhysicalKey::Code(KeyCode::Delete) => Key::SpecialKey(SpecialKey::Delete),

        PhysicalKey::Code(KeyCode::Enter) => Key::SpecialKey(SpecialKey::Enter),
        PhysicalKey::Code(KeyCode::Backspace) => Key::SpecialKey(SpecialKey::Backspace),
        PhysicalKey::Code(KeyCode::NumpadBackspace) => Key::SpecialKey(SpecialKey::Backspace),

        PhysicalKey::Code(KeyCode::Home) => Key::SpecialKey(SpecialKey::Home),
        PhysicalKey::Code(KeyCode::End) => Key::SpecialKey(SpecialKey::End),
        PhysicalKey::Code(KeyCode::PageUp) => Key::SpecialKey(SpecialKey::PageUp),
        PhysicalKey::Code(KeyCode::PageDown) => Key::SpecialKey(SpecialKey::PageDown),

        PhysicalKey::Code(KeyCode::ArrowLeft) => Key::SpecialKey(SpecialKey::Left),
        PhysicalKey::Code(KeyCode::ArrowRight) => Key::SpecialKey(SpecialKey::Right),
        PhysicalKey::Code(KeyCode::ArrowUp) => Key::SpecialKey(SpecialKey::Up),
        PhysicalKey::Code(KeyCode::ArrowDown) => Key::SpecialKey(SpecialKey::Down),

        PhysicalKey::Code(KeyCode::Tab) => Key::SpecialKey(SpecialKey::Tab),

        _ => Key::Unknown,
    };
    match special_key {
        Key::Unknown => {
            if let Some(repr) = key.logical_key.to_text() {
                Key::Character(repr.to_string())
            } else {
                Key::Unknown
            }
        }
        _ => special_key,
    }
}

pub const fn cursor_icon(icon: CursorIcon) -> window::CursorIcon {
    match icon {
        CursorIcon::Default => window::CursorIcon::Default,
        CursorIcon::ContextMenu => window::CursorIcon::ContextMenu,
        CursorIcon::Help => window::CursorIcon::Help,
        CursorIcon::Pointer => window::CursorIcon::Pointer,
        CursorIcon::Progress => window::CursorIcon::Progress,
        CursorIcon::Wait => window::CursorIcon::Wait,
        CursorIcon::Cell => window::CursorIcon::Cell,
        CursorIcon::Crosshair => window::CursorIcon::Crosshair,
        CursorIcon::Text => window::CursorIcon::Text,
        CursorIcon::VerticalText => window::CursorIcon::VerticalText,
        CursorIcon::Alias => window::CursorIcon::Alias,
        CursorIcon::Copy => window::CursorIcon::Copy,
        CursorIcon::Move => window::CursorIcon::Move,
        CursorIcon::NoDrop => window::CursorIcon::NoDrop,
        CursorIcon::NotAllowed => window::CursorIcon::NotAllowed,
        CursorIcon::Grab => window::CursorIcon::Grab,
        CursorIcon::Grabbing => window::CursorIcon::Grabbing,
        CursorIcon::EResize => window::CursorIcon::EResize,
        CursorIcon::NResize => window::CursorIcon::NResize,
        CursorIcon::NeResize => window::CursorIcon::NeResize,
        CursorIcon::NwResize => window::CursorIcon::NwResize,
        CursorIcon::SResize => window::CursorIcon::SResize,
        CursorIcon::SeResize => window::CursorIcon::SeResize,
        CursorIcon::SwResize => window::CursorIcon::SwResize,
        CursorIcon::WResize => window::CursorIcon::WResize,
        CursorIcon::EwResize => window::CursorIcon::EwResize,
        CursorIcon::NsResize => window::CursorIcon::NsResize,
        CursorIcon::NeswResize => window::CursorIcon::NeswResize,
        CursorIcon::NwseResize => window::CursorIcon::NwseResize,
        CursorIcon::ColResize => window::CursorIcon::ColResize,
        CursorIcon::RowResize => window::CursorIcon::RowResize,
        CursorIcon::AllScroll => window::CursorIcon::AllScroll,
        CursorIcon::ZoomIn => window::CursorIcon::ZoomIn,
        CursorIcon::ZoomOut => window::CursorIcon::ZoomOut,
    }
}

// Parts of this is derived from https://github.com/Smithay/client-toolkit input events.
use bitflags::bitflags;
use euclid::default::Point2D;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "sctk")]
pub mod sctk;

/// A unique identifier for a pointer.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PointerId(u64);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum InputMethod {
    Keyboard,
    Mouse,
    Tablet,
    Touch,
}

bitflags! {
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Modifiers: u32 {
        const SHIFT = 1 << 0;
        const CTRL = 1 << 1;
        const ALT = 1 << 2;
        /// The "logo" key, also known as the "windows" or "super" key on a keyboard.
        #[doc(alias = "windows")]
        #[doc(alias = "super")]
        const LOGO = 1 << 3;

        const CAPS_LOCK = 1 << 4;
        const NUM_LOCK = 1 << 5;
    }

}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyboardEvent {
    pub modifiers: Modifiers,
    pub kind: KeyboardEventKind,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum KeyboardEventKind {
    Press(Key),
    Release(Key),
    ModifiersChanged,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    SpecialKey(SpecialKey),
    Character(String),
    Unknown,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpecialKey {
    LShift,
    LCtrl,
    LAlt,
    RShift,
    RCtrl,
    RAlt,

    Logo,

    CapsLock,
    NumLock,

    Escape,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,

    Insert,
    PrintScreen,
    Delete,

    Enter,
    Backspace,

    Home,
    End,
    PageUp,
    PageDown,

    Left,
    Right,
    Up,
    Down,

    Tab,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,

    /// The fourth non-scroll button, which is often used as "back" in web browsers.
    Side,
    /// The fifth non-scroll button, which is often used as "forward" in web browsers.
    Extra,

    Forward,
    Back,
    Task,
}

// Basically copy smithay-client-toolkit's types here, since for other platforms to not have to
// depend on it,
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MouseEvent {
    pub position: Point2D<f32>,
    pub kind: MouseEventKind,
}

impl MouseEvent {
    pub const fn new(position: Point2D<f32>, kind: MouseEventKind) -> Self {
        Self { position, kind }
    }
    pub const fn enter(position: Point2D<f32>) -> Self {
        Self::new(position, MouseEventKind::Enter)
    }
    pub const fn leave(position: Point2D<f32>) -> Self {
        Self::new(position, MouseEventKind::Leave)
    }
}
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MouseEventKind {
    Enter,
    Leave,
    Motion {
        time: u32,
    },
    Press {
        time: u32,
        button: MouseButton,
    },
    Release {
        time: u32,
        button: MouseButton,
    },
    Axis {
        time: u32,
        horizontal: AxisScroll,
        vertical: AxisScroll,
        source: Option<AxisSource>,
    },
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AxisScroll {
    /// The scroll measured in pixels.
    pub absolute: f64,

    /// The scroll measured in steps.
    ///
    /// Note: this might always be zero if the scrolling is due to a touchpad or other continuous
    /// source.
    pub discrete: i32,

    /// The scroll was stopped.
    ///
    /// Generally this is encountered when hardware indicates the end of some continuous scrolling.
    pub stop: bool,
}

/// Describes the source types for axis events. This indicates to the
/// client how an axis event was physically generated; a client may
/// adjust the user interface accordingly. For example, scroll events
/// from a "finger" source may be in a smooth coordinate space with
/// kinetic scrolling whereas a "wheel" source may be in discrete steps
/// of a number of lines.
///
/// The "continuous" axis source is a device generating events in a
/// continuous coordinate space, but using something other than a
/// finger. One example for this source is button-based scrolling where
/// the vertical motion of a device is converted to scroll events while
/// a button is held down.
///
/// The "wheel tilt" axis source indicates that the actual device is a
/// wheel but the scroll event is not caused by a rotation but a
/// (usually sideways) tilt of the wheel.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum AxisSource {
    Wheel,
    Finger,
    Continuous,
    WheelTilt,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub enum CursorIcon {
    /// The platform-dependent default cursor. Often rendered as arrow.
    #[default]
    Default,

    /// A context menu is available for the object under the cursor. Often
    /// rendered as an arrow with a small menu-like graphic next to it.
    ContextMenu,

    /// Help is available for the object under the cursor. Often rendered as a
    /// question mark or a balloon.
    Help,

    /// The cursor is a pointer that indicates a link. Often rendered as the
    /// backside of a hand with the index finger extended.
    Pointer,

    /// A progress indicator. The program is performing some processing, but is
    /// different from [`CursorIcon::Wait`] in that the user may still interact
    /// with the program.
    Progress,

    /// Indicates that the program is busy and the user should wait. Often
    /// rendered as a watch or hourglass.
    Wait,

    /// Indicates that a cell or set of cells may be selected. Often rendered as
    /// a thick plus-sign with a dot in the middle.
    Cell,

    /// A simple crosshair (e.g., short line segments resembling a "+" sign).
    /// Often used to indicate a two dimensional bitmap selection mode.
    Crosshair,

    /// Indicates text that may be selected. Often rendered as an I-beam.
    Text,

    /// Indicates vertical-text that may be selected. Often rendered as a
    /// horizontal I-beam.
    VerticalText,

    /// Indicates an alias of/shortcut to something is to be created. Often
    /// rendered as an arrow with a small curved arrow next to it.
    Alias,

    /// Indicates something is to be copied. Often rendered as an arrow with a
    /// small plus sign next to it.
    Copy,

    /// Indicates something is to be moved.
    Move,

    /// Indicates that the dragged item cannot be dropped at the current cursor
    /// location. Often rendered as a hand or pointer with a small circle with a
    /// line through it.
    NoDrop,

    /// Indicates that the requested action will not be carried out. Often
    /// rendered as a circle with a line through it.
    NotAllowed,

    /// Indicates that something can be grabbed (dragged to be moved). Often
    /// rendered as the backside of an open hand.
    Grab,

    /// Indicates that something is being grabbed (dragged to be moved). Often
    /// rendered as the backside of a hand with fingers closed mostly out of
    /// view.
    Grabbing,

    /// The east border to be moved.
    EResize,

    /// The north border to be moved.
    NResize,

    /// The north-east corner to be moved.
    NeResize,

    /// The north-west corner to be moved.
    NwResize,

    /// The south border to be moved.
    SResize,

    /// The south-east corner to be moved.
    SeResize,

    /// The south-west corner to be moved.
    SwResize,

    /// The west border to be moved.
    WResize,

    /// The east and west borders to be moved.
    EwResize,

    /// The south and north borders to be moved.
    NsResize,

    /// The north-east and south-west corners to be moved.
    NeswResize,

    /// The north-west and south-east corners to be moved.
    NwseResize,

    /// Indicates that the item/column can be resized horizontally. Often
    /// rendered as arrows pointing left and right with a vertical bar
    /// separating them.
    ColResize,

    /// Indicates that the item/row can be resized vertically. Often rendered as
    /// arrows pointing up and down with a horizontal bar separating them.
    RowResize,

    /// Indicates that the something can be scrolled in any direction. Often
    /// rendered as arrows pointing up, down, left, and right with a dot in the
    /// middle.
    AllScroll,

    /// Indicates that something can be zoomed in. Often rendered as a
    /// magnifying glass with a "+" in the center of the glass.
    ZoomIn,

    /// Indicates that something can be zoomed in. Often rendered as a
    /// magnifying glass with a "-" in the center of the glass.
    ZoomOut,
}

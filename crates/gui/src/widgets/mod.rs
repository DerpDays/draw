use euclid::default::{Point2D, Vector2D};
use graphics::{Mesh, Systems, get_empty_mesh};

use crate::Element;
use crate::Vertex;
use crate::events::{BlurEvent, ChangeEvent, EventContext, FocusEvent};
use crate::macros::widget::{delegate_widget, impl_as_variants};
use input::{KeyboardEvent, MouseEvent};

pub mod background;
pub mod button;
pub mod container;
pub mod slider;
pub mod svg;
pub mod text;
pub mod text_input;

pub use background::BackgroundWidget;
pub use background::TransitionBackgroundWidget;
pub use button::ButtonWidget;
pub use container::ContainerWidget;
pub use slider::SliderWidget;
pub use svg::SvgWidget;
pub use text::TextWidget;
pub use text_input::TextInputWidget;

#[derive(Default)]
pub enum Widget<M: Clone> {
    #[default]
    Layout,
    Other(Box<dyn Element<Message = M>>),

    Background(BackgroundWidget<M>),
    Button(ButtonWidget<M>),
    Container(ContainerWidget<M>),
    Slider(SliderWidget<M>),
    Svg(SvgWidget<M>),
    Text(TextWidget<M>),
    TextInput(TextInputWidget<M>),
    TransitionBackground(TransitionBackgroundWidget<M>),
}

impl_as_variants! {
    container => Container(ContainerWidget<M>),
    button => Button(ButtonWidget<M>),
    svg => Svg(SvgWidget<M>),
    background => Background(BackgroundWidget<M>),
    transition_background => TransitionBackground(TransitionBackgroundWidget<M>),
    text_input => TextInput(TextInputWidget<M>),
    slider => Slider(SliderWidget<M>),
}

impl<M: Clone> Element for Widget<M> {
    type Message = M;

    fn as_widget(self) -> Widget<M> {
        self
    }

    delegate_widget!(
        Background,
        Button,
        Container,
        Slider,
        Svg,
        Text,
        TextInput,
        TransitionBackground
    );
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct WidgetInteractionState {
    active: bool,

    pressed: bool,
    hovered: bool,

    enabled: bool,
}
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WidgetVisualState {
    /// Pressed is the state with the most visual priority, it represents that the widget
    /// is currently being pressed by a left mouse click.
    Pressed = 0,
    /// Active is a special state for widgets that can be toggled on/off at the user's discretion.
    /// This is meant to represent when a widget is in a special state such as being the current
    /// tool, menu currently selected, or otherwise.
    Active = 1,
    /// Represents that the mouse is currently inside this widget (or mouse events are being shared
    /// to it).
    Hovered = 2,
    /// The normal state of the widget when it has neither mouse or keyboard focus.
    Normal = 3,
    /// A special state for when the widget is marked as disabled.
    Disabled = 4,
}
impl WidgetInteractionState {
    pub const fn to_visual(&self) -> WidgetVisualState {
        if !self.enabled {
            return WidgetVisualState::Disabled;
        }
        if self.pressed {
            return WidgetVisualState::Pressed;
        }
        if self.active {
            return WidgetVisualState::Active;
        }
        if self.hovered {
            return WidgetVisualState::Hovered;
        }
        WidgetVisualState::Normal
    }
}

impl WidgetInteractionState {
    pub const fn new(active: bool, clicked: bool, hovered: bool, enabled: bool) -> Self {
        Self {
            active,
            pressed: clicked,
            hovered,
            enabled,
        }
    }
    pub const fn empty() -> Self {
        Self::new(false, false, false, false)
    }
}

pub(crate) enum LayoutChange {
    Translate(Vector2D<f32>),
    Rerender,
}

pub(crate) fn parse_layout_change(
    new_layout: taffy::Layout,
    prev_layout: taffy::Layout,
) -> LayoutChange {
    let mut new_no_loc = new_layout;
    new_no_loc.location = taffy::Point::zero();

    let mut prev_no_loc = prev_layout;
    prev_no_loc.location = taffy::Point::zero();

    // if only the location changed between these
    if new_no_loc == prev_no_loc {
        LayoutChange::Translate(
            Point2D::new(new_layout.location.x, new_layout.location.y)
                - Point2D::new(prev_layout.location.x, prev_layout.location.y),
        )
    } else {
        LayoutChange::Rerender
    }
}

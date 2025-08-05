// TODO: Should probably not require this, and hence nightly.
#![feature(macro_metavar_expr_concat)]

//! # This is a small library for creating user interfaces in rust.
//!
//! This works upon the assumption that you have your own renderer.
//!
//!
//!
//! ### Feature flags:
//! - `widgets` (*enabled by default*) - Enables the built in widget library, this is
//!   recommended to avoid having to implement your own interactivity.

use crate::events::{BlurEvent, ChangeEvent, EventContext, FocusEvent};

use graphics::{Mesh, Systems, Vertex};
use input::{KeyboardEvent, MouseEvent};
use widgets::Widget;

mod macros;

mod events;
pub mod tree;
pub mod widgets;

pub use tree::UITree;

pub mod prelude {
    pub use crate::events::*;
    pub use crate::events::{EventContext, EventPhase, HandlesEvent, Redraw};
    pub use crate::Element;
    pub use taffy::prelude::*;
}

pub mod reexports {
    pub use taffy;
}

#[allow(unused_variables)]
pub trait Element {
    type Message: Clone;

    fn as_widget(self) -> Widget<Self::Message>;

    fn render(&mut self, systems: &mut Systems, layout: taffy::Layout) -> &Mesh<Vertex>;
    fn measure(
        &mut self,
        systems: &mut Systems,
        available_space: taffy::Size<taffy::AvailableSpace>,
        style: &taffy::Style,
    ) -> taffy::Size<f32> {
        taffy::Size::zero()
    }

    fn is_dirty(&self) -> bool;
    fn clear_cache(&mut self);
    fn focusable(&self) -> bool;

    fn mouse_event(&mut self, ctx: &mut EventContext<MouseEvent, Self::Message>);
    fn keyboard_event(&mut self, ctx: &mut EventContext<KeyboardEvent, Self::Message>);

    /// Triggered whenever the element gains focus.
    fn focus_event(&mut self, ctx: &mut EventContext<FocusEvent, Self::Message>) {}
    /// Triggered whenever the element goes out of focus.
    fn blur_event(&mut self, ctx: &mut EventContext<BlurEvent, Self::Message>) {}

    fn change_event_f32(&mut self, ctx: &mut EventContext<ChangeEvent<f32>, Self::Message>) {}
    fn change_event_string(&mut self, ctx: &mut EventContext<ChangeEvent<String>, Self::Message>) {}
}

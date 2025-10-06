#![feature(const_trait_impl)]

pub mod canvas;
pub mod pipeline;
pub mod projection;
pub mod scene;
pub mod tools;
pub mod view;

pub mod ui;
pub mod ui2;

pub use gui_reactive::{AnimationHandle, AnimationManager};

use std::time::Duration;

use euclid::default::Point2D;
use serde::{Deserialize, Serialize};

use graphics::{Drawable, Vertex};

#[derive(Deserialize, Serialize)]
pub enum ClickResult {
    Handled,
    Unhandled,
}

/// A trait for shapes that are drawable, have a unique ID, a color, and a layer order.
pub trait Node: Drawable<Vertex> {
    fn handle_click(&mut self, position: Point2D<f32>) -> ClickResult;
    fn handle_drag(&mut self, position: Point2D<f32>);
}

pub trait RedrawRequest {
    fn request_redraw(&self);
    fn request_redraw_duration(&self, duration: Duration);
}
pub trait RedrawRequestV2 {
    fn request_redraw(&self);
    fn new_animation_handle(&self) -> AnimationHandle;
}

#![feature(const_trait_impl)]

pub mod canvas;
pub mod pipeline;
pub mod projection;
pub mod scene;
pub mod tools;
pub mod view;

pub mod ui;

use std::time::Duration;

use lyon::math::Point;
use serde::{Deserialize, Serialize};

use graphics::{Drawable, Vertex};

#[derive(Deserialize, Serialize)]
pub enum ClickResult {
    Handled,
    Unhandled,
}

/// A trait for shapes that are drawable, have a unique ID, a color, and a layer order.
pub trait Node: Drawable<Vertex> {
    fn handle_click(&mut self, position: Point) -> ClickResult;
    fn handle_drag(&mut self, position: Point);
}

pub trait RedrawRequest {
    fn request_redraw(&self);
    fn request_redraw_duration(&self, duration: Duration);
}

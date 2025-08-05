use color::{PremulColor, Srgb};
use euclid::default::Point2D;
use graphics::{CanvasCoordinates, Mesh, Primitive, Systems, Vertex};

use gui::reexports::taffy::NodeId;
use input::{CursorIcon, KeyboardEvent, Modifiers, MouseEvent};

use crate::projection::Projection;

mod arrow;
mod ellipse;
mod eraser;
mod grab;
mod highlighter;
mod line;
mod pen;
mod rectangle;
mod select;
mod text;
mod zoom;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, strum::EnumIter)]
pub enum ToolKind {
    Grab,
    Select,
    Pen,
    Line,
    Arrow,
    #[default]
    Rectangle,
    Ellipse,
    Text,
    Highlighter,
    Eraser,
    Zoom,
}
#[derive(Clone, Debug, Default)]
pub struct Tools {
    pub grab: grab::GrabTool,
    pub select: select::SelectTool,
    pub pen: pen::PenTool,
    pub line: line::LineTool,
    pub arrow: arrow::ArrowTool,
    pub rectangle: rectangle::RectangleTool,
    pub ellipse: ellipse::EllipseTool,
    pub text: text::TextTool,
    pub highlighter: highlighter::HighlighterTool,
    pub eraser: eraser::EraserTool,
    pub zoom: zoom::ZoomTool,
}

#[derive(Clone, Copy, Debug)]
pub struct ToolNodeMap {
    grab: NodeId,
    select: NodeId,
    pen: NodeId,
    line: NodeId,
    arrow: NodeId,
    rectangle: NodeId,
    ellipse: NodeId,
    text: NodeId,
    highlighter: NodeId,
    eraser: NodeId,
    zoom: NodeId,
}
impl ToolNodeMap {
    pub const fn zero() -> Self {
        Self {
            grab: NodeId::new(0),
            select: NodeId::new(0),
            pen: NodeId::new(0),
            line: NodeId::new(0),
            arrow: NodeId::new(0),
            rectangle: NodeId::new(0),
            ellipse: NodeId::new(0),
            text: NodeId::new(0),
            highlighter: NodeId::new(0),
            eraser: NodeId::new(0),
            zoom: NodeId::new(0),
        }
    }
    pub const fn set(&mut self, kind: ToolKind, node_id: NodeId) {
        match kind {
            ToolKind::Grab => self.grab = node_id,
            ToolKind::Select => self.select = node_id,
            ToolKind::Pen => self.pen = node_id,
            ToolKind::Line => self.line = node_id,
            ToolKind::Arrow => self.arrow = node_id,
            ToolKind::Rectangle => self.rectangle = node_id,
            ToolKind::Ellipse => self.ellipse = node_id,
            ToolKind::Text => self.text = node_id,
            ToolKind::Highlighter => self.highlighter = node_id,
            ToolKind::Eraser => self.eraser = node_id,
            ToolKind::Zoom => self.zoom = node_id,
        }
    }
}

impl ToolKind {
    pub fn svg_icon(&self) -> &[u8] {
        match self {
            ToolKind::Grab => include_bytes!("../../../../resources/grab.svg"),
            ToolKind::Select => include_bytes!("../../../../resources/select.svg"),
            ToolKind::Pen => include_bytes!("../../../../resources/pen.svg"),
            ToolKind::Line => include_bytes!("../../../../resources/line.svg"),
            ToolKind::Arrow => include_bytes!("../../../../resources/arrow.svg"),
            ToolKind::Rectangle => include_bytes!("../../../../resources/rectangle.svg"),
            ToolKind::Ellipse => include_bytes!("../../../../resources/ellipse.svg"),
            ToolKind::Text => include_bytes!("../../../../resources/text.svg"),
            ToolKind::Highlighter => include_bytes!("../../../../resources/highlighter.svg"),
            ToolKind::Eraser => include_bytes!("../../../../resources/eraser.svg"),
            ToolKind::Zoom => include_bytes!("../../../../resources/zoom.svg"),
        }
    }
    pub const fn default_cursor(&self) -> CursorIcon {
        match self {
            ToolKind::Grab => CursorIcon::Grab,
            ToolKind::Select => CursorIcon::Default,
            ToolKind::Eraser => CursorIcon::NotAllowed,
            ToolKind::Zoom => CursorIcon::ZoomIn,
            ToolKind::Text => CursorIcon::Text,
            _ => CursorIcon::Crosshair,
        }
    }

    pub const fn get_node_id(&self, node_map: &ToolNodeMap) -> NodeId {
        match self {
            ToolKind::Grab => node_map.grab,
            ToolKind::Select => node_map.select,
            ToolKind::Pen => node_map.pen,
            ToolKind::Line => node_map.line,
            ToolKind::Arrow => node_map.arrow,
            ToolKind::Rectangle => node_map.rectangle,
            ToolKind::Ellipse => node_map.ellipse,
            ToolKind::Text => node_map.text,
            ToolKind::Highlighter => node_map.highlighter,
            ToolKind::Eraser => node_map.eraser,
            ToolKind::Zoom => node_map.zoom,
        }
    }
    pub fn mouse_event(
        &self,
        systems: &mut Systems,
        tools: &mut Tools,
        event: MouseEvent,
        modifiers: Modifiers,
        projection: &Projection,
    ) -> Vec<ToolMessage> {
        let event = MouseEvent {
            position: projection.viewport_to_world(event.position),
            ..event
        };
        match self {
            ToolKind::Grab => tools.grab.mouse_event(systems, event, modifiers),
            ToolKind::Select => tools.select.mouse_event(systems, event, modifiers),
            ToolKind::Pen => tools.pen.mouse_event(systems, event, modifiers),
            ToolKind::Line => tools.line.mouse_event(systems, event, modifiers),
            ToolKind::Arrow => tools.arrow.mouse_event(systems, event, modifiers),
            ToolKind::Rectangle => tools.rectangle.mouse_event(systems, event, modifiers),
            ToolKind::Ellipse => tools.ellipse.mouse_event(systems, event, modifiers),
            ToolKind::Text => tools.text.mouse_event(systems, event, modifiers),
            ToolKind::Highlighter => tools.highlighter.mouse_event(systems, event, modifiers),

            ToolKind::Eraser => tools.eraser.mouse_event(systems, event, modifiers),
            ToolKind::Zoom => tools.zoom.mouse_event(systems, event, modifiers),
        }
    }

    pub fn keyboard_event(
        &self,
        systems: &mut Systems,
        tools: &mut Tools,
        event: KeyboardEvent,
    ) -> Vec<ToolMessage> {
        match self {
            ToolKind::Grab => tools.grab.keyboard_event(systems, event),
            ToolKind::Select => tools.select.keyboard_event(systems, event),
            ToolKind::Pen => tools.pen.keyboard_event(systems, event),
            ToolKind::Line => tools.line.keyboard_event(systems, event),
            ToolKind::Arrow => tools.arrow.keyboard_event(systems, event),
            ToolKind::Rectangle => tools.rectangle.keyboard_event(systems, event),
            ToolKind::Ellipse => tools.ellipse.keyboard_event(systems, event),
            ToolKind::Text => tools.text.keyboard_event(systems, event),
            ToolKind::Highlighter => tools.highlighter.keyboard_event(systems, event),
            ToolKind::Eraser => tools.eraser.keyboard_event(systems, event),
            ToolKind::Zoom => tools.zoom.keyboard_event(systems, event),
        }
    }
}

pub trait Tool {
    fn mouse_event(
        &mut self,
        systems: &mut Systems,
        event: MouseEvent,
        modifiers: Modifiers,
    ) -> Vec<ToolMessage>;

    #[allow(unused_variables)]
    fn keyboard_event(&mut self, systems: &mut Systems, event: KeyboardEvent) -> Vec<ToolMessage> {
        vec![]
    }
}

#[derive(Debug)]
pub enum ToolMessage {
    CursorIcon(CursorIcon),
    Commit(Primitive<CanvasCoordinates>),
    Scratch(Mesh<Vertex>),
    ClearScratch,
    ChangePrimaryColor(PremulColor<Srgb>),

    SetFocus,
    ReleaseFocus,

    Select(Point2D<f32>),
    GrabMove(Point2D<f32>, Point2D<f32>),
    Erase(Point2D<f32>),

    ZoomIn(Point2D<f32>),
    ZoomOut(Point2D<f32>),
    ResetZoom,
}

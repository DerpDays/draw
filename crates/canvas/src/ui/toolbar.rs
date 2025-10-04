use color::AlphaColor;
use euclid::default::Point2D;
use graphics::{Rounding, primitives::RectangleOptions};

use gui::prelude::*;

use gui::Element;
use gui::tree::UITree;
use gui::widgets::button::{ButtonOptions, FADE_DURATION};
use gui::widgets::{BackgroundWidget, ButtonWidget, ContainerWidget, SvgWidget, Widget};
use input::{CursorIcon, MouseButton, MouseEventKind};

use crate::tools::ToolKind;
use crate::ui::Message;
use crate::ui::styles::{colors, floating_grab};

pub struct Toolbar {
    active_tool: ToolKind,
    visibility_container: NodeId,
    grab_container: NodeId,

    grab: ToolButton,
    select: ToolButton,
    pen: ToolButton,
    line: ToolButton,
    arrow: ToolButton,
    rectangle: ToolButton,
    ellipse: ToolButton,
    text: ToolButton,
    highlighter: ToolButton,
    eraser: ToolButton,
    zoom: ToolButton,
}

impl Toolbar {
    pub fn visibility_container(&self) -> NodeId {
        self.visibility_container
    }
    fn to_button(&self, tool: ToolKind) -> &ToolButton {
        match tool {
            ToolKind::Grab => &self.grab,
            ToolKind::Select => &self.select,
            ToolKind::Pen => &self.pen,
            ToolKind::Line => &self.line,
            ToolKind::Arrow => &self.arrow,
            ToolKind::Rectangle => &self.rectangle,
            ToolKind::Ellipse => &self.ellipse,
            ToolKind::Text => &self.text,
            ToolKind::Highlighter => &self.highlighter,
            ToolKind::Eraser => &self.eraser,
            ToolKind::Zoom => &self.zoom,
        }
    }

    pub fn swap_tool(&mut self, tree: &mut UITree<Widget<Message>>, tool: ToolKind) {
        if self.active_tool != tool {
            self.to_button(self.active_tool).set_active(tree, false);
            self.to_button(tool).set_active(tree, true);
            self.active_tool = tool;
        }
    }

    pub fn set_visibility(&self, tree: &mut UITree<Widget<Message>>, visible: bool) {
        tree.set_style(
            self.visibility_container,
            Style {
                display: if visible {
                    Default::default()
                } else {
                    Display::None
                },
                ..Style::DEFAULT
            },
        );
    }

    pub fn build(
        tree: &mut UITree<Widget<Message>>,
        initial_position: Point2D<f32>,
        active_tool: ToolKind,
    ) -> Self {
        let visibility_container = tree.new_leaf(Widget::Layout, Style::DEFAULT);

        let grab_container = tree.new_leaf(
            ContainerWidget::new(true)
                .mouse_handler(crate::ui::grab_fn)
                .as_widget(),
            floating_grab(100., initial_position),
        );

        let toolbar = tree.new_leaf(
            BackgroundWidget::new(RectangleOptions {
                color: colors::BACKGROUND.into(),
                rounding: Rounding::all(5.),
                stroke_width: 3.,
                stroke_color: colors::BORDER.into(),
                box_sizing: graphics::BoxSizing::ContentBox,
            })
            .as_widget(),
            Style {
                display: Display::Flex,
                padding: Rect::<LengthPercentage>::length(5.),
                gap: Size::<LengthPercentage>::length(5.),
                ..Default::default()
            },
        );

        tree.add_child(visibility_container, grab_container);
        tree.add_child(grab_container, toolbar);

        // Initialise tool buttons
        let grab = ToolButton::build(tree, toolbar, ToolKind::Grab, active_tool);
        let select = ToolButton::build(tree, toolbar, ToolKind::Select, active_tool);
        let pen = ToolButton::build(tree, toolbar, ToolKind::Pen, active_tool);
        let line = ToolButton::build(tree, toolbar, ToolKind::Line, active_tool);
        let arrow = ToolButton::build(tree, toolbar, ToolKind::Arrow, active_tool);
        let rectangle = ToolButton::build(tree, toolbar, ToolKind::Rectangle, active_tool);
        let ellipse = ToolButton::build(tree, toolbar, ToolKind::Ellipse, active_tool);
        let text = ToolButton::build(tree, toolbar, ToolKind::Text, active_tool);
        let highlighter = ToolButton::build(tree, toolbar, ToolKind::Highlighter, active_tool);
        let eraser = ToolButton::build(tree, toolbar, ToolKind::Eraser, active_tool);
        let zoom = ToolButton::build(tree, toolbar, ToolKind::Zoom, active_tool);

        Self {
            active_tool,
            visibility_container,
            grab_container,

            grab,
            select,
            pen,
            line,
            arrow,
            rectangle,
            ellipse,
            text,
            highlighter,
            eraser,
            zoom,
        }
    }
}

pub struct ToolButton {
    container: NodeId,
}
impl ToolButton {
    fn set_active(&self, tree: &mut UITree<Widget<Message>>, active: bool) {
        tree.get_node_mut(self.container)
            .as_button_mut()
            .expect("container is a button node")
            .set_active(active);
    }
    fn build(
        tree: &mut UITree<Widget<Message>>,
        parent: NodeId,
        tool: ToolKind,
        active_tool: ToolKind,
    ) -> Self {
        let tool_style = Style {
            display: Display::Flex,
            justify_content: Some(AlignContent::Center),
            align_items: Some(AlignItems::Stretch),
            size: Size::<Dimension>::from_lengths(48., 48.),
            ..Default::default()
        };
        let container = tree.new_leaf(
            ButtonWidget::new(
                ButtonOptions {
                    pressed: RectangleOptions {
                        color: AlphaColor::new([0.32, 0.32, 0.32, 1.]).into(),
                        rounding: Rounding::all(5.),
                        ..Default::default()
                    },
                    active: RectangleOptions {
                        color: AlphaColor::new([0.25, 0.25, 0.25, 1.]).into(),
                        rounding: Rounding::all(5.),
                        ..Default::default()
                    },
                    hovered: RectangleOptions {
                        color: AlphaColor::new([0.2, 0.2, 0.2, 1.]).into(),
                        rounding: Rounding::all(5.),
                        ..Default::default()
                    },
                    normal: RectangleOptions {
                        color: colors::BACKGROUND.into(),
                        rounding: Rounding::all(5.),
                        ..Default::default()
                    },
                    disabled: RectangleOptions {
                        color: AlphaColor::new([0.3, 0.14, 0.14, 0.5]).into(),
                        rounding: Rounding::all(5.),
                        ..Default::default()
                    },
                },
                true,
                tool == active_tool,
                FADE_DURATION,
            )
            .mouse_handler(move |_, ctx| {
                match ctx.payload().kind {
                    MouseEventKind::Enter => {
                        ctx.push_messages(vec![Message::CursorIcon(CursorIcon::Pointer)])
                    }
                    MouseEventKind::Press { button, .. } if button == MouseButton::Left => {
                        ctx.push_messages(vec![Message::SwapTool(tool)])
                    }
                    _ => {}
                };
                match ctx.current_phase() {
                    EventPhase::Direct | EventPhase::AtTarget | EventPhase::Bubbling => {
                        ctx.stop_propagation();
                    }
                    _ => {}
                }
            })
            .as_widget(),
            tool_style,
        );

        let svg_node = tree.new_leaf(
            SvgWidget::new(
                tool.svg_icon().into(),
                gui::widgets::svg::SvgOptions {
                    normal: graphics::primitives::SvgOptions {
                        fill_color: Some(colors::WHITE),
                        stroke_color: Some(colors::WHITE),
                        ..Default::default()
                    },
                    hover: None,
                },
            )
            .as_widget(),
            Style {
                flex_grow: 1.,
                margin: Rect::length(8.),
                ..Default::default()
            },
        );

        tree.add_child(container, svg_node);
        tree.add_child(parent, container);
        Self { container }
    }
}

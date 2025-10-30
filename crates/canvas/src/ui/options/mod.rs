use color::AlphaColor;
use euclid::default::Point2D;
use graphics::{
    Rounding,
    primitives::{RectangleOptions, TextOptions},
};

use gui::{
    prelude::*,
    tree::{UITree, ZIndexProperties},
    widgets::{BackgroundWidget, ContainerWidget, TextWidget, Widget},
};
use input::{CursorIcon, MouseEventKind};

use crate::{
    tools::ToolKind,
    ui::{
        Message,
        options::{color_picker::ColorPickerTree, color_swatches::ColorSwatches},
        styles::{colors, floating_grab},
    },
};

mod color_picker;
mod color_swatches;
mod rectangle;

#[derive(Copy, Clone, Debug)]
pub enum OptionsMessage {
    ColorPicker(color_picker::ColorPickerMessage),
    SelectSwatch(usize),
}

impl From<OptionsMessage> for Message {
    fn from(value: OptionsMessage) -> Self {
        Message::ToolOptions(value)
    }
}

pub struct OptionsTree {
    pub grab_area_node: NodeId,
    pub background_node: NodeId,
    pub swatches: color_swatches::ColorSwatches<4>,
    pub color_picker: ColorPickerTree,

    is_shown: bool,
}

impl OptionsTree {
    pub fn swap_tool(&mut self, tool: ToolKind) {
        let should_show = match tool {
            ToolKind::Grab => false,
            ToolKind::Select => false,
            ToolKind::Eraser => false,
            ToolKind::Zoom => false,
            _ => true,
        };
        if self.is_shown && !should_show {
        } else if !self.is_shown && should_show {
        };
    }

    pub fn update(&mut self, tree: &mut UITree<Widget<Message>>, message: &OptionsMessage) {
        match message {
            OptionsMessage::ColorPicker(message) => {
                self.color_picker.update(tree, message);
                self.swatches.update_swatch(
                    tree,
                    self.swatches.selected_swatch,
                    self.color_picker.color.convert(),
                );
            }
            OptionsMessage::SelectSwatch(swatch_id) => {
                if self.swatches.selected_swatch == *swatch_id {
                    // toggle the color picker
                    if tree.get_style(self.color_picker.container).display != Display::None {
                        tree.set_style(
                            self.color_picker.container,
                            color_picker::color_picker_container_style(),
                        );
                    } else {
                        tree.set_style(
                            self.color_picker.container,
                            Style {
                                display: Display::Flex,
                                ..color_picker::color_picker_container_style()
                            },
                        );
                    }
                } else {
                    self.swatches.update_selected(tree, *swatch_id);
                    self.color_picker.update(
                        tree,
                        &color_picker::ColorPickerMessage::UpdateColor(
                            self.swatches.swatch_colors[*swatch_id..*swatch_id + 1][0],
                        ),
                    );
                }
            }
        }
    }
    pub fn build(tree: &mut UITree<Widget<Message>>, root: NodeId) -> Self {
        let grab_area = ContainerWidget::new(true).mouse_handler(crate::ui::grab_fn);
        let grab_area_node = tree.new_leaf_with_z(
            grab_area.as_widget(),
            floating_grab(100., Point2D::new(100., 500.)),
            ZIndexProperties {
                z_index: 1,
                isolate_z: true,
            },
        );
        tree.add_child(root, grab_area_node);

        let container = tree.new_leaf(
            ContainerWidget::new(false)
                .mouse_handler(|_, ctx| {
                    if !ctx.in_capture_phase() && ctx.payload().kind == MouseEventKind::Enter {
                        ctx.push_messages(vec![Message::CursorIcon(CursorIcon::Default)]);
                    }
                })
                .as_widget(),
            Style::DEFAULT,
        );
        tree.add_child(grab_area_node, container);

        let background_node = tree.new_leaf(
            BackgroundWidget::new(RectangleOptions {
                color: colors::BACKGROUND.into(),
                rounding: Rounding::all(5.),
                stroke_width: 3.,
                stroke_color: AlphaColor::new([0.9, 0.9, 0.9, 1.]).into(),
                ..RectangleOptions::DEFAULT
            })
            .mouse_handler(|_, ctx| {
                if !ctx.in_capture_phase() {
                    ctx.stop_propagation();
                }
            })
            .as_widget(),
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                padding: Rect::<LengthPercentage>::length(20.),
                gap: Size::<LengthPercentage>::length(15.),
                ..Style::DEFAULT
            },
        );
        tree.add_child(container, background_node);

        let color_picker_label = tree.new_leaf(
            TextWidget::new(
                "Rectangle Tool".to_string(),
                TextOptions {
                    color: AlphaColor::new([1., 1., 1., 1.]),
                    font_size: 16.,
                    ..Default::default()
                },
            )
            .as_widget(),
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                padding: Rect {
                    // bottom: LengthPercentage::length(10.),
                    ..Rect::zero()
                },
                ..Style::DEFAULT
            },
        );
        tree.add_child(background_node, color_picker_label);
        let color_picker = ColorPickerTree::build(tree);

        let swatches_container = tree.new_leaf(
            Widget::Layout,
            Style {
                display: Display::Flex,
                align_items: Some(AlignItems::Center),
                gap: Size::length(10.),
                ..Style::DEFAULT
            },
        );

        tree.add_child(background_node, swatches_container);

        let swatches_label = tree.new_leaf(
            TextWidget::new(
                "Fill Color: ".to_string(),
                TextOptions {
                    color: AlphaColor::new([1., 1., 1., 1.]),
                    font_size: 13.,
                    ..Default::default()
                },
            )
            .as_widget(),
            Style::DEFAULT,
        );

        tree.add_child(swatches_container, swatches_label);
        let swatches = ColorSwatches::build(
            tree,
            &[
                AlphaColor::WHITE,
                AlphaColor::BLACK,
                AlphaColor::new([1., 0., 0., 1.]),
                AlphaColor::new([0., 1., 0., 1.]),
            ],
            0,
        );
        tree.add_child(swatches_container, swatches.container);
        tree.add_child(background_node, color_picker.container);

        Self {
            grab_area_node,
            background_node,
            swatches,
            color_picker,
            is_shown: true,
        }
    }
}

pub enum OptionsType {
    ColorSwatches,
    Float {
        label: Option<&'static str>,
    },
    IconRadio {
        label: Option<&'static str>,
        options: Vec<RadioOption>,
    },
}
impl OptionsType {
    pub fn build_ui<T>(
        &self,
        tree: &mut UITree<Widget<Message>>,
        val_fn: fn(T) -> Message,
    ) -> NodeId {
        match self {
            OptionsType::ColorSwatches => ColorPickerTree::build(tree).container,
            OptionsType::Float { label } => todo!(),
            OptionsType::IconRadio { label, options } => todo!(),
        }
    }
}

pub struct RadioOption {
    label: &'static str,
    icon: &'static [u8],
}

enum ToolOptionsUpdate {
    Line(LineOptionsUpdate),
}

struct LineSettings {
    color: AlphaColor<color::Srgb>,
    width: f32,
}
impl LineSettings {}

enum LineOptionsUpdate {
    UpdateColor(AlphaColor<color::Srgb>),
    UpdateWidth(f32),
}
impl From<LineOptionsUpdate> for Message {
    fn from(value: LineOptionsUpdate) -> Self {
        // Self::ToolOptions(ToolOptionsUpdate::Line(value))
        todo!()
    }
}

// struct ArrowSettings {
//     color: AlphaColor<color::Srgb>,
//     width: f32,
//
//     line_snap_angle: f32,
// }
// struct RectangleSettings {
//     color: AlphaColor<color::Srgb>,
//     width: f32,
//
//     line_snap_angle: f32,
// }
// struct EllipseSettings {
//     color: AlphaColor<color::Srgb>,
//     width: f32,
//
//     line_snap_angle: f32,
// }
//
// struct TextSettings {
//     options: graphics::primitives::TextOptions,
// }
//
// struct HighlighterSettings {
//     color: AlphaColor<color::Srgb>,
//     width: f32,
//
//     line_snap_angle: f32,
// }

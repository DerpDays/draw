use color::{AlphaColor, Srgb};
use graphics::{BasicColor, Rounding, primitives::RectangleOptions};
use gui::{
    UITree,
    prelude::*,
    widgets::{BackgroundWidget, Widget},
};
use input::{MouseButton, MouseEventKind};

use crate::ui::{Message, options::OptionsMessage, styles::colors};

pub struct ColorSwatches<const N: usize> {
    pub container: NodeId,
    pub selected_swatch: usize,

    swatch_nodes: [NodeId; N],
    pub swatch_colors: [AlphaColor<Srgb>; N],
    swatch_containers: [NodeId; N],
}

const ACTIVE_BORDER: AlphaColor<Srgb> = colors::BORDER_SELECTED;
const INACTIVE_BORDER: AlphaColor<Srgb> = colors::TRANSPARENT;
const BORDER_WIDTH: f32 = 1.;
const BORDER_ROUNDING: f32 = 5.;
const SWATCH_ROUNDING: f32 = 5.;

impl<const N: usize> ColorSwatches<N> {
    pub fn update_swatch(
        &mut self,
        tree: &mut UITree<Widget<Message>>,
        id: usize,
        color: AlphaColor<Srgb>,
    ) {
        self.swatch_colors[id] = color;
        tree.get_node_mut(self.swatch_nodes[id])
            .as_background_mut()
            .expect("swatch nodes can only be of type background")
            .change_options(RectangleOptions {
                color: color.into(),
                rounding: Rounding::all(SWATCH_ROUNDING),
                ..RectangleOptions::DEFAULT
            });
    }

    pub fn update_selected(&mut self, tree: &mut UITree<Widget<Message>>, id: usize) {
        tree.get_node_mut(self.swatch_containers[self.selected_swatch])
            .as_background_mut()
            .expect("swatch nodes can only be of type background")
            .change_options(RectangleOptions {
                color: colors::TRANSPARENT.into(),
                stroke_color: INACTIVE_BORDER.into(),
                stroke_width: BORDER_WIDTH,
                rounding: Rounding::all(BORDER_ROUNDING),
                ..RectangleOptions::DEFAULT
            });
        self.selected_swatch = id;
        tree.get_node_mut(self.swatch_containers[self.selected_swatch])
            .as_background_mut()
            .expect("swatch nodes can only be of type background")
            .change_options(RectangleOptions {
                color: colors::BLACK.into(),
                stroke_color: ACTIVE_BORDER.into(),
                stroke_width: BORDER_WIDTH,
                rounding: Rounding::all(BORDER_ROUNDING),
                ..RectangleOptions::DEFAULT
            });
    }

    pub fn build(
        tree: &mut UITree<Widget<Message>>,
        swatches: &[AlphaColor<Srgb>; N],
        selected: usize,
    ) -> ColorSwatches<N> {
        let container = tree.new_leaf(
            Widget::Layout,
            Style {
                display: Display::Flex,
                gap: Size::length(10.),
                ..Style::DEFAULT
            },
        );

        let swatch_containers = std::array::from_fn(|i| {
            let node = tree.new_leaf(
                BackgroundWidget::new(RectangleOptions {
                    color: if i == selected {
                        colors::BLACK.into()
                    } else {
                        colors::TRANSPARENT.into()
                    },
                    stroke_color: if i == selected {
                        ACTIVE_BORDER.into()
                    } else {
                        INACTIVE_BORDER.into()
                    },
                    stroke_width: BORDER_WIDTH,
                    rounding: Rounding::all(BORDER_ROUNDING),
                    ..RectangleOptions::DEFAULT
                })
                .as_widget(),
                Style {
                    display: Display::Flex,
                    justify_items: Some(JustifyItems::Center),
                    align_items: Some(AlignItems::Center),
                    size: Size::length(25.),
                    padding: Rect::length(3.),
                    ..Style::DEFAULT
                },
            );
            tree.add_child(container, node);
            node
        });

        let swatch_nodes = std::array::from_fn(|i| {
            let node = tree.new_leaf(
                BackgroundWidget::new(RectangleOptions {
                    color: BasicColor::Solid(swatches[i]),
                    rounding: Rounding::all(SWATCH_ROUNDING),
                    ..RectangleOptions::DEFAULT
                })
                .mouse_handler(move |_, ctx| {
                    if !ctx.in_capture_phase() {
                        match ctx.payload().kind {
                            MouseEventKind::Press { button, .. } if button == MouseButton::Left => {
                                ctx.push_messages(vec![OptionsMessage::SelectSwatch(i).into()]);
                                ctx.request_redraw(Redraw::Now);
                            }
                            _ => {}
                        }
                    }
                })
                .as_widget(),
                Style {
                    size: Size::percent(1.),
                    ..Style::DEFAULT
                },
            );
            tree.add_child(swatch_containers[i], node);
            node
        });

        Self {
            container,
            selected_swatch: 0,

            swatch_nodes,
            swatch_colors: *swatches,
            swatch_containers,
        }
    }
}

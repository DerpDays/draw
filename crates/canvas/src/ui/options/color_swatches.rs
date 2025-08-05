use color::{PremulColor, Srgb};
use graphics::{primitives::RectangleOptions, BasicColor, Rounding};
use gui::{
    prelude::*,
    widgets::{BackgroundWidget, Widget},
    UITree,
};

use crate::ui::{styles::colors, Message};

pub struct ColorSwatches {
    pub container: NodeId,
    swatches: [PremulColor<Srgb>; 4],
    swatch_nodes: [NodeId; 4],
    color_picker_button: NodeId,
}

impl ColorSwatches {
    pub fn build(
        tree: &mut UITree<Widget<Message>>,
        swatches: &[PremulColor<Srgb>; 4],
    ) -> ColorSwatches {
        let container = tree.new_leaf(
            Widget::Layout,
            Style {
                display: Display::Flex,
                padding: Rect::length(10.),
                gap: Size::length(10.),
                ..Style::DEFAULT
            },
        );

        let swatch_nodes = std::array::from_fn(|i| {
            let node = tree.new_leaf(
                BackgroundWidget::new(RectangleOptions {
                    color: BasicColor::Solid(swatches[i]),
                    stroke_color: colors::BORDER.into(),
                    stroke_width: 1.,
                    rounding: Rounding::all(5.),
                    ..RectangleOptions::DEFAULT
                })
                .as_widget(),
                Style {
                    display: Display::Flex,
                    size: Size::length(25.),
                    gap: Size::length(10.),
                    ..Style::DEFAULT
                },
            );
            tree.add_child(container, node);
            node
        });

        let separator = tree.new_leaf(
            BackgroundWidget::new(RectangleOptions::only_color(colors::FOREGROUND)).as_widget(),
            Style {
                size: Size {
                    width: Dimension::length(2.),
                    height: Dimension::percent(1.),
                },
                ..Style::DEFAULT
            },
        );
        tree.add_child(container, separator);

        let node = tree.new_leaf(
            BackgroundWidget::new(RectangleOptions {
                color: BasicColor::Solid(PremulColor::new([0.4, 0.4, 0.2, 1.])),
                stroke_color: colors::BORDER.into(),
                stroke_width: 1.,
                rounding: Rounding::all(5.),
                ..RectangleOptions::DEFAULT
            })
            .as_widget(),
            Style {
                display: Display::Flex,
                size: Size::length(25.),
                gap: Size::length(10.),
                ..Style::DEFAULT
            },
        );
        tree.add_child(container, node);

        Self {
            container,
            swatches: *swatches,
            swatch_nodes,
            color_picker_button: NodeId::new(0),
        }
    }
}

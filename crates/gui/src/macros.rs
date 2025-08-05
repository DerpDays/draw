pub(crate) mod widget {
    macro_rules! impl_as_variants {
        (
            $( $fn_name:ident => $variant:ident ( $widget_type:ty ) ),* $(,)?
        ) => {
            impl<M: Clone> Widget<M> {
                $(
                    pub const fn ${concat(as_, $fn_name)}(&self) -> Option<&$widget_type> {
                        match self {
                            Widget::$variant(inner) => Some(inner),
                            _ => None,
                        }
                    }
                    pub const fn ${concat(as_, $fn_name, _mut)}(&mut self) -> Option<&mut $widget_type> {
                        match self {
                            Widget::$variant(inner) => Some(inner),
                            _ => None,
                        }
                    }
                )*
            }
        };
    }

    macro_rules! delegate_widget {
        (
            $( $variant:ident ),*
        ) => {
            fn render(&mut self, systems: &mut Systems, layout: taffy::Layout) -> &Mesh<Vertex> {
                match self {
                    $( Widget::$variant(w) => w.render(systems, layout), )*

                    Widget::Other(w) => w.render(systems, layout),
                    Widget::Layout => get_empty_mesh(),
                }
            }

            fn measure(
                &mut self,
                systems: &mut Systems,
                available_space: taffy::Size<taffy::AvailableSpace>,
                style: &taffy::Style,
            ) -> taffy::Size<f32> {
                match self {
                    $( Widget::$variant(w) => w.measure(systems, available_space, style), )*

                    Widget::Other(w) => w.measure(systems, available_space, style),
                    Widget::Layout => taffy::Size::zero(),
                }
            }

            fn is_dirty(&self) -> bool {
                match self {
                    $( Widget::$variant(w) => w.is_dirty(), )*

                    Widget::Other(w) => w.is_dirty(),
                    Widget::Layout => false,
                }
            }

            fn clear_cache(&mut self) {
                match self {
                    $( Widget::$variant(w) => w.clear_cache(), )*

                    Widget::Other(w) => w.clear_cache(),
                    Widget::Layout => {},
                }
            }

            fn focusable(&self) -> bool {
                match self {
                    $( Widget::$variant(w) => w.focusable(), )*

                    Widget::Other(w) => w.focusable(),
                    Widget::Layout => false,
                }
            }


            fn mouse_event(&mut self, ctx: &mut EventContext<MouseEvent, Self::Message>) {
                match self {
                    $( Widget::$variant(w) => w.mouse_event(ctx), )*

                    Widget::Other(w) => w.mouse_event(ctx),
                    Widget::Layout => {},
                }
            }

            fn keyboard_event(&mut self, ctx: &mut EventContext<KeyboardEvent, Self::Message>) {
                match self {
                    $( Widget::$variant(w) => w.keyboard_event(ctx), )*

                    Widget::Other(w) => w.keyboard_event(ctx),
                    Widget::Layout => {},
                }
            }

            fn focus_event(&mut self, ctx: &mut EventContext<FocusEvent, Self::Message>) {
                match self {
                    $( Widget::$variant(w) => w.focus_event(ctx), )*

                    Widget::Other(w) => w.focus_event(ctx),
                    Widget::Layout => {},
                }
            }

            fn blur_event(&mut self, ctx: &mut EventContext<BlurEvent, Self::Message>) {
                match self {
                    $( Widget::$variant(w) => w.blur_event(ctx), )*

                    Widget::Other(w) => w.blur_event(ctx),
                    Widget::Layout => {},
                }
            }

            fn change_event_f32(&mut self, ctx: &mut EventContext<ChangeEvent<f32>, Self::Message>) {
                match self {
                    $( Widget::$variant(w) => w.change_event_f32(ctx), )*

                    Widget::Other(w) => w.change_event_f32(ctx),
                    Widget::Layout => {},
                }
            }

            fn change_event_string(&mut self, ctx: &mut EventContext<ChangeEvent<String>, Self::Message>) {
                match self {
                    $( Widget::$variant(w) => w.change_event_string(ctx), )*

                    Widget::Other(w) => w.change_event_string(ctx),
                    Widget::Layout => {},
                }
            }

        };
    }

    // Allow usage in the crate without re-exporting it
    pub(crate) use {delegate_widget, impl_as_variants};
}

pub(crate) mod event_handlers {
    macro_rules! handle_event_doc {
        ($event: ty, $handler_fn: ident) => {
            concat!(
                "This widget supports the handling of `",
                stringify!($event),
                "` through a blanket implementation of [this trait](crate::prelude::",
                stringify!($event),
                "Handler).\n\nYou can attach a handler like this:\n```rust\nwidget.",
                stringify!($handler_fn),
                "(|this, ctx| { .. });\n```"
            )
        };
    }

    /// Internal macro to generate HandlesEvent impls with documentation.
    macro_rules! impl_event_handler {
        (
            $widget:ident,
            $( $event:ty => $field_name:ident ),* $(,)?
        ) => {
            $(

                #[doc = $crate::macros::event_handlers::handle_event_doc!($event, $field_name)]
                impl<M: Clone> $crate::prelude::HandlesEvent<$event> for $widget<M> {

                        fn handler_mut(&mut self) -> &mut crate::prelude::EventHandler<$event, Self> {
                            &mut self.$field_name
                        }
                }
            )*
        };
    }

    pub(crate) use {handle_event_doc, impl_event_handler};
}

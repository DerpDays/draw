use crate::tree::{Element, ElementWithChildren, Node, NodeForEach, Widget};

use next_tuple::NextTuple;

pub trait ElementChild<N>
where
    N: Node,
{
    /// The type of the element, with the child added.
    type Output;

    /// Adds a child to an element.
    fn child(self, child: N) -> Self::Output;
}

impl<T, C, N> ElementChild<N> for Element<T, C>
where
    T: ElementWithChildren + Widget,
    C: NodeForEach + NextTuple,
    <C as NextTuple>::Output<N>: NodeForEach,
    N: Node,
{
    type Output = Element<T, <C as NextTuple>::Output<N>>;

    fn child(self, child: N) -> Self::Output {
        Element {
            style: self.style,
            final_layout: self.final_layout,
            unrounded_layout: self.unrounded_layout,
            cache: self.cache,

            zindex: self.zindex,

            inner: self.inner,
            children: self.children.next_tuple(child),

            focusable: self.focusable,

            mouse_handler: self.mouse_handler,
            keyboard_handler: self.keyboard_handler,
            focus_handler: self.focus_handler,
            blur_handler: self.blur_handler,
        }
    }
}

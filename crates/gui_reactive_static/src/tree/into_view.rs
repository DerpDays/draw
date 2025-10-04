use crate::tree::Node;

pub struct View<T: Node> {
    pub inner: T,
}

pub trait IntoView
where
    Self: Sized + Node + Send,
{
    /// Wraps the inner type.
    fn into_view(self) -> View<Self>;
}

impl<T> IntoView for T
where
    T: Sized + Node + Send,
{
    fn into_view(self) -> View<T> {
        View { inner: self }
    }
}

// use crate::widgets::{Build, Element};
//
// /// A wrapper for any kind of view.
// #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
// pub struct View<T>
// where
//     T: Sized,
// {
//     inner: T,
// }
//
// impl<T> View<T> {
//     /// Wraps the view.
//     pub fn new(inner: T) -> Self {
//         Self { inner }
//     }
//
//     /// Unwraps the view, returning the inner type.
//     pub fn into_inner(self) -> T {
//         self.inner
//     }
// }
//
// /// A trait that is implemented for types that can be rendered.
// pub trait IntoView
// where
//     Self: Sized + Build + Send,
// {
//     /// Wraps the inner type.
//     fn into_view(self) -> View<Self>;
// }
//
// impl<T> IntoView for T
// where
//     T: Sized + Build + Send,
// {
//     fn into_view(self) -> View<Self> {
//         View { inner: self }
//     }
// }
//
// impl<T: Build> Build for View<T> {
//     type State = T::State;
//
//     fn build(self) -> Self::State {
//         self.inner.build()
//     }
//
//     fn rebuild(self, state: &mut Self::State) {
//         self.inner.rebuild(state)
//     }
// }
//
// /// Collects some iterator of views into a list, so they can be rendered.
// ///
// /// This is a shorthand for `.collect::<Vec<_>>()`, and allows any iterator of renderable
// /// items to be collected into a renderable collection.
// pub trait CollectView {
//     /// The inner view type.
//     type View: IntoView;
//
//     /// Collects the iterator into a list of views.
//     fn collect_view(self) -> Vec<Self::View>;
// }
//
// impl<It, V> CollectView for It
// where
//     It: IntoIterator<Item = V>,
//     V: IntoView,
// {
//     type View = V;
//
//     fn collect_view(self) -> Vec<Self::View> {
//         self.into_iter().collect()
//     }
// }
//
// impl Build for () {
//     type State = ();
//
//     fn build(self) -> Self::State {
//         ()
//     }
//
//     fn rebuild(self, _state: &mut Self::State) {}
// }
// impl<A: Build> Build for (A,) {
//     type State = A::State;
//
//     fn build(self) -> Self::State {
//         self.0.build()
//     }
//
//     fn rebuild(self, state: &mut Self::State) {
//         self.0.rebuild(state)
//     }
// }
//
// macro_rules! impl_view_for_tuples {
//     ($first:ident, $($ty:ident),* $(,)?) => {
//         impl<$first, $($ty),*> Build for ($first, $($ty,)*)
//         where
//             $first: Build,
//             $($ty: Build),*,
//
//         {
//             type State = ($first::State, $($ty::State,)*);
//
//             fn build(self) -> Self::State {
//                 #[allow(non_snake_case)]
//                 let ($first, $($ty,)*) = self;
//                 (
//                     $first.build(),
//                     $($ty.build()),*
//                 )
//             }
//
//             fn rebuild(self, state: &mut Self::State) {
//                 paste::paste! {
//                     let ([<$first:lower>], $([<$ty:lower>],)*) = self;
//                     let ([<view_ $first:lower>], $([<view_ $ty:lower>],)*) = state;
//                     [<$first:lower>].rebuild([<view_ $first:lower>]);
//                     $([<$ty:lower>].rebuild([<view_ $ty:lower>]));*
//                 }
//             }
//         }
//     };
// }
//
// impl_view_for_tuples!(A, B);
// impl_view_for_tuples!(A, B, C);
// impl_view_for_tuples!(A, B, C, D);
// impl_view_for_tuples!(A, B, C, D, E);
// impl_view_for_tuples!(A, B, C, D, E, F);
// impl_view_for_tuples!(A, B, C, D, E, F, G);
// impl_view_for_tuples!(A, B, C, D, E, F, G, H);
// impl_view_for_tuples!(A, B, C, D, E, F, G, H, I);
// impl_view_for_tuples!(A, B, C, D, E, F, G, H, I, J);
// impl_view_for_tuples!(A, B, C, D, E, F, G, H, I, J, K);
// impl_view_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L);
// impl_view_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M);
// impl_view_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
// impl_view_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
// impl_view_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
// impl_view_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q);
// impl_view_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R);
// impl_view_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S);
// impl_view_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T);
// impl_view_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U);
// impl_view_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V);
// impl_view_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W);
// impl_view_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X);
// impl_view_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y);
// impl_view_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);

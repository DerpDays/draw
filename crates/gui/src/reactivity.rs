pub use sycamore_reactive::{
    NodeHandle,
    ReadSignal,
    RootHandle,
    Signal,
    Trackable,
    batch,
    create_child_scope,
    create_effect,
    create_effect_initial,
    create_memo,
    create_reducer,
    create_root,
    create_selector,
    create_selector_with,
    create_signal,
    map_indexed,
    map_keyed,
    on,
    on_cleanup,
    provide_context,
    try_use_context,
    untrack,
    use_context,
    use_context_or_else,
    use_current_scope,
    use_global_scope,
    use_scope_depth,
};

use std::{borrow::Cow, rc::Rc};

/// Represents a value that can be either static or dynamic.
///
/// This is useful for cases where you want to accept a value that can be either static or dynamic,
/// such as in component props.
///
/// A [`MaybeDyn`] value can be created from a static value or a closure that returns the value by
/// using the [`From`] trait.
///
/// # Creating a `MaybeDyn`
///
/// You can create a `MaybeDyn` from a static value by using the [`MaybeDyn::Static`] variant.
/// However, most of the times, you probably want to use the implementation of the `From<U>` trait
/// for `MaybeDyn<T>`.
///
/// This trait is already implemented globally for signals and closures that return `T`. However,
/// we cannot provide a blanket implementation for all types `T` to convert into `MaybeDyn<T>`
/// because of specialization. Instead, we can only implement it for specific types.
#[derive(Clone)]
pub enum MaybeDyn<T>
where
    T: Into<Self> + 'static,
{
    /// A static value.
    Static(T),
    /// A dynamic value backed by a signal.
    Signal(ReadSignal<T>),
    /// A derived dynamic value.
    Derived(Rc<dyn Fn() -> Self>),
}

impl<T: Into<Self> + 'static> MaybeDyn<T> {
    /// Get the value by consuming itself. Unlike [`get_clone`](Self::get_clone), this method avoids
    /// a clone if we are just storing a static value.
    pub fn evaluate(self) -> T
    where
        T: Clone,
    {
        match self {
            Self::Static(value) => value,
            Self::Signal(signal) => signal.get_clone(),
            Self::Derived(f) => f().evaluate(),
        }
    }

    /// Get a value from inside the [`MaybeDyn`].
    ///
    /// When called inside a reactive scope, any underlying signal will be automatically tracked.
    pub fn with<U>(&self, f: impl FnOnce(&T) -> U) -> U {
        match self {
            Self::Static(value) => (f)(value),
            Self::Signal(value) => value.with(f),
            Self::Derived(d) => d().with(f),
        }
    }

    /// Get the value by copying it.
    ///
    /// If the type does not implement [`Copy`], consider using [`get_clone`](Self::get_clone)
    /// instead.
    pub fn get(&self) -> T
    where
        T: Copy,
    {
        match self {
            Self::Static(value) => *value,
            Self::Signal(value) => value.get(),
            Self::Derived(f) => f().evaluate(),
        }
    }

    /// Get the value by cloning it.
    ///
    /// If the type implements [`Copy`], consider using [`get`](Self::get) instead.
    pub fn get_clone(&self) -> T
    where
        T: Clone,
    {
        match self {
            Self::Static(value) => value.clone(),
            Self::Signal(value) => value.get_clone(),
            Self::Derived(f) => f().evaluate(),
        }
    }

    /// Get a value without tracking it.
    pub fn with_untracked<U>(&self, f: impl FnOnce(&T) -> U) -> U {
        match self {
            Self::Static(value) => (f)(value),
            Self::Signal(value) => value.with_untracked(f),
            Self::Derived(d) => d().with_untracked(f),
        }
    }

    /// Get the value without tracking it. The type must implement [`Copy`].
    /// If this is not the case, use [`MaybeDyn::get_clone_untracked`] or [`MaybeDyn::with_untracked`] instead.
    pub fn get_untracked(&self) -> T
    where
        T: Copy,
    {
        match self {
            Self::Static(value) => *value,
            Self::Signal(value) => value.get_untracked(),
            Self::Derived(f) => f().get_untracked(),
        }
    }

    /// Get the value without tracking it. The type is [`Clone`]-ed automatically.
    ///
    /// This is the cloned equivalent of [`MaybeDyn::get_untracked`].
    pub fn get_clone_untracked(&self) -> T
    where
        T: Clone,
    {
        match self {
            Self::Static(value) => value.clone(),
            Self::Signal(value) => value.get_clone_untracked(),
            Self::Derived(f) => f().get_clone_untracked(),
        }
    }

    /// Track the reactive dependencies, if it is dynamic.
    pub fn track(&self) {
        match self {
            Self::Static(_) => {}
            Self::Signal(signal) => signal.track(),
            Self::Derived(f) => f().track(),
        }
    }

    /// Tries to get the value statically or returns `None` if value is dynamic.
    pub fn as_static(&self) -> Option<&T> {
        match self {
            Self::Static(value) => Some(value),
            _ => None,
        }
    }
}

impl<T: Into<Self>, U: Into<MaybeDyn<T>> + Clone> From<ReadSignal<U>> for MaybeDyn<T> {
    fn from(val: ReadSignal<U>) -> Self {
        // Check if U == T, i.e. ReadSignal<U> is actually a ReadSignal<T>.
        //
        // If so, we use a trick to convert the generic type to the concrete type. This should be
        // optimized out by the compiler to be zero-cost.
        if let Some(val) =
            (&mut Some(val) as &mut dyn std::any::Any).downcast_mut::<Option<ReadSignal<T>>>()
        {
            MaybeDyn::Signal(val.unwrap())
        } else {
            MaybeDyn::Derived(Rc::new(move || val.get_clone().into()))
        }
    }
}

impl<T: Into<Self>, U: Into<MaybeDyn<T>> + Clone> From<Signal<U>> for MaybeDyn<T> {
    fn from(val: Signal<U>) -> Self {
        Self::from(*val)
    }
}

#[diagnostic::do_not_recommend]
impl<F, U, T: Into<Self>> From<F> for MaybeDyn<T>
where
    F: Fn() -> U + 'static,
    U: Into<MaybeDyn<T>>,
{
    fn from(f: F) -> Self {
        MaybeDyn::Derived(Rc::new(move || f().into()))
    }
}

/// A macro that makes it easy to write implementations for `Into<MaybeDyn<T>>`.
///
/// Because of Rust orphan rules, you can only implement `Into<MaybeDyn<T>>` for types that are
/// defined in the current crate. To work around this limitation, the newtype pattern can be used.
///
/// # Example
///
/// ```
/// # use sycamore_reactive::*;
///
/// struct MyType;
///
/// struct OtherType;
///
/// impl From<OtherType> for MyType {
///     fn from(_: OtherType) -> Self {
///         todo!();
///     }
/// }
///
/// // You can also list additional types that can be converted to `MaybeDyn<MyType>`.
/// impl_into_maybe_dyn!(MyType; OtherType);
/// ```
macro_rules! impl_into_maybe_dyn {
    ($ty:ty $(; $($from:ty),*)?) => {
        impl From<$ty> for $crate::reactivity::MaybeDyn<$ty> {
            fn from(val: $ty) -> Self {
                MaybeDyn::Static(val)
            }
        }

        impl_into_maybe_dyn_with_convert!($ty; Into::into $(; $($from),*)?);
    };
}

/// Create `From<U>` implementations for `MaybeDyn<T>` for a list of types.
///
/// Usually, you would use the [`impl_into_maybe_dyn!`] macro instead of this macro.
macro_rules! impl_into_maybe_dyn_with_convert {
    ($ty:ty; $convert:expr $(; $($from:ty),*)?) => {
        $(
            $(
                impl From<$from> for $crate::reactivity::MaybeDyn<$ty> {
                    fn from(val: $from) -> Self {
                        MaybeDyn::Static($convert(val))
                    }
                }
            )*
        )?
    };
}

impl_into_maybe_dyn!(Cow<'static, str>; &'static str, String);
impl_into_maybe_dyn_with_convert!(
    Option<Cow<'static, str>>; |x| Some(Into::into(x));
    Cow<'static, str>, &'static str, String
);
impl_into_maybe_dyn_with_convert!(
    Option<Cow<'static, str>>; |x| Option::map(x, Into::into);
    Option<&'static str>, Option<String>
);

impl_into_maybe_dyn!(bool);

impl_into_maybe_dyn!(f32);
impl_into_maybe_dyn!(f64);

impl_into_maybe_dyn!(i8);
impl_into_maybe_dyn!(i16);
impl_into_maybe_dyn!(i32);
impl_into_maybe_dyn!(i64);
impl_into_maybe_dyn!(i128);
impl_into_maybe_dyn!(isize);
impl_into_maybe_dyn!(u8);
impl_into_maybe_dyn!(u16);
impl_into_maybe_dyn!(u32);
impl_into_maybe_dyn!(u64);
impl_into_maybe_dyn!(u128);
impl_into_maybe_dyn!(usize);
impl_into_maybe_dyn!(std::time::Duration);

impl<T> From<Option<T>> for MaybeDyn<Option<T>> {
    fn from(val: Option<T>) -> Self {
        MaybeDyn::Static(val)
    }
}

impl<T> From<Vec<T>> for MaybeDyn<Vec<T>> {
    fn from(val: Vec<T>) -> Self {
        MaybeDyn::Static(val)
    }
}

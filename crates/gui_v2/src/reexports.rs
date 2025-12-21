pub mod reactive {
    pub use sycamore_reactive::*;

    pub fn maybe_get_untracked<T: Copy + Into<MaybeDyn<T>>>(maybe_dyn: &MaybeDyn<T>) -> T {
        match maybe_dyn {
            MaybeDyn::Static(val) => *val,
            MaybeDyn::Signal(signal) => signal.get_untracked(),
            MaybeDyn::Derived(derived) => maybe_get_untracked(&derived()),
        }
    }
    pub fn maybe_get_clone_untracked<T: Clone + Into<MaybeDyn<T>>>(maybe_dyn: &MaybeDyn<T>) -> T {
        match maybe_dyn {
            MaybeDyn::Static(val) => val.clone(),
            MaybeDyn::Signal(signal) => signal.get_clone_untracked(),
            MaybeDyn::Derived(derived) => maybe_get_clone_untracked(&derived()),
        }
    }
}
pub use taffy;

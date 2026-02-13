#[cfg(not(target_arch = "wasm32"))]
pub type Instant = std::time::Instant;

#[cfg(target_arch = "wasm32")]
use core::time::Duration;

#[cfg(target_arch = "wasm32")]
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
/// milliseconds from performance.now()
pub struct Instant(pub u64);

#[cfg(target_arch = "wasm32")]
fn get_wasm_now() -> u64 {
    thread_local! {
        static PERF: web_sys::Performance = web_sys::window()
            .unwrap()
            .performance()
            .unwrap();
    }
    PERF.with(|p| p.now() as u64)
}

#[cfg(target_arch = "wasm32")]
impl Instant {
    #[inline]
    pub fn now() -> Self {
        Self(get_wasm_now())
    }

    #[inline]
    pub fn elapsed(self) -> Duration {
        Duration::from_millis((get_wasm_now() - self.0).max(0))
    }
}

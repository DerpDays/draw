#[cfg(not(target_arch = "wasm32"))]
pub type Instant = std::time::Instant;

#[cfg(target_arch = "wasm32")]
use core::time::Duration;

#[cfg(target_arch = "wasm32")]
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct Instant {
    t: f64, // milliseconds from performance.now()
}
#[cfg(target_arch = "wasm32")]
fn get_wasm_now() -> f64 {
    thread_local! {
        static PERF: web_sys::Performance = web_sys::window()
            .unwrap()
            .performance()
            .unwrap();
    }
    PERF.with(|p| p.now())
}

#[cfg(target_arch = "wasm32")]
impl Instant {
    #[inline]
    pub fn now() -> Self {
        Self { t: get_wasm_now() }
    }

    #[inline]
    pub fn elapsed(self) -> Duration {
        let dt = get_wasm_now() - self.t;
        Duration::from_secs_f64((dt.max(0.0)) / 1000.0)
    }
}

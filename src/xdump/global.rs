use std::sync::atomic::{AtomicBool, Ordering};

use once_cell::sync::Lazy;

static CAPTURE_STATE: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

pub fn on_capture_state() {
    CAPTURE_STATE.store(true, Ordering::SeqCst);
}

pub fn off_capture_state() {
    CAPTURE_STATE.store(false, Ordering::SeqCst);
}

pub fn check_capture_state() -> bool {
    CAPTURE_STATE.load(Ordering::SeqCst)
}

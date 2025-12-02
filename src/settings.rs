use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize},
};

// use atomic var to reduce the runner thread to restart
#[derive(Clone, Debug)]
pub struct Settings {
    pub enable_eq: Arc<AtomicBool>,
    pub instance_id: Arc<AtomicUsize>,
    pub latency: u32,
}

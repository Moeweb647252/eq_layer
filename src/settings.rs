use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize},
};

use crate::eq::EqProfile;

// use atomic var to reduce the runner thread to restart
#[derive(Clone, Debug)]
pub struct Settings {
    pub enable_eq: Arc<AtomicBool>,
    pub eq_profile: EqProfile,
    pub instance_id: Arc<AtomicUsize>,
    pub latency: u32,
}

use crate::{settings::Settings, ui::command::oneshot::OneShot};

pub mod oneshot {
    use std::{
        fmt::Debug,
        sync::{Arc, atomic::AtomicBool},
    };

    struct Inner<T> {
        pub has_value: AtomicBool,
        pub value: *mut Option<T>,
    }

    unsafe impl<T: Send> Send for Inner<T> {}
    unsafe impl<T: Sync> Sync for Inner<T> {}

    #[derive(Clone)]
    pub struct OneShot<T>(Arc<Inner<T>>);

    impl<T> Debug for OneShot<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("OneShot")
        }
    }

    impl<T> OneShot<T> {
        pub fn new() -> Self {
            let value = Box::into_raw(Box::new(None));
            Self(Arc::new(Inner {
                has_value: AtomicBool::new(false),
                value,
            }))
        }

        pub fn send(self, value: T) {
            unsafe {
                *(*self.0).value = Some(value);
                self.0
                    .has_value
                    .store(true, std::sync::atomic::Ordering::Release);
            }
        }

        pub fn recv(&self) -> T {
            while !self.0.has_value.load(std::sync::atomic::Ordering::Acquire) {
                std::thread::yield_now();
            }
            unsafe { (*self.0.value).take().unwrap() }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct State {
    pub enabled: bool,
    pub running: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            enabled: true,
            running: true,
        }
    }
}

pub struct Info {
    pub device_names: Vec<String>,
    pub input_dev: String,
    pub output_dev: String,
}

#[derive(Debug)]
pub enum SetDevice {
    Input,
    Output,
}

#[derive(Debug)]
pub enum Command {
    SetState(State),
    UpdateSettings(Settings),
    GetState(OneShot<State>),
    SetDevice(SetDevice, String),
}

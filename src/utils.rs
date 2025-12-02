use std::ops::{Deref, DerefMut};

pub use oneshot::OneShot;

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

pub struct DerefMutHook<T> {
    data: T,
    call: Option<Box<dyn Fn(&T)>>,
}

impl<T> DerefMutHook<T> {
    pub fn new(data: T) -> Self {
        Self { data, call: None }
    }

    pub fn set_callback(&mut self, callback: impl Fn(&T) + 'static) {
        self.call = Some(Box::new(callback));
    }

    pub fn remove_hook(&mut self) {
        self.call = None;
    }
}

impl<T> DerefMut for DerefMutHook<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if let Some(call) = self.call.as_ref() {
            call(&self.data);
        }
        &mut self.data
    }
}

impl<T> Deref for DerefMutHook<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

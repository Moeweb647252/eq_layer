use std::os::raw::c_void;

use block2::StackBlock;
use eframe::egui;
use objc2::rc::Retained;
use objc2::runtime::NSObject;
use objc2::{class, msg_send};
use tracing::debug;

/// Registers for `NSApplicationDidBecomeActiveNotification`.
/// This fires when the user clicks the app's dock icon or Cmd-Tabs to the app.
///
/// Returns an observer token. Keep it alive to stay registered; dropping it
/// removes the observer from the notification center.
pub fn setup_dock_observer(ctx: egui::Context) -> Retained<NSObject> {
    let block: block2::RcBlock<dyn Fn(*mut NSObject)> =
        StackBlock::new(move |_notification: *mut NSObject| {
            debug!("Received NSApplicationDidBecomeActiveNotification");
            ctx.request_repaint();
        })
        .copy();

    unsafe {
        let center: *mut NSObject = msg_send![class!(NSNotificationCenter), defaultCenter];

        // Build an NSString for the notification name
        let name_str = "NSApplicationDidBecomeActiveNotification";
        let ns_name: *mut NSObject = msg_send![class!(NSString), alloc];
        let ns_name: *mut NSObject = msg_send![
            ns_name,
            initWithBytes: name_str.as_ptr() as *const c_void,
            length: name_str.len(),
            encoding: 4u64, // NSUTF8StringEncoding
        ];

        // addObserverForName:object:queue:usingBlock: returns a retained observer
        let observer: Retained<NSObject> = msg_send![
            center,
            addObserverForName: &*ns_name,
            object: std::ptr::null_mut::<NSObject>(),
            queue: std::ptr::null_mut::<NSObject>(),
            usingBlock: &*block,
        ];

        observer
    }
}

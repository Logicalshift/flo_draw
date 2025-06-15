use objc2::*;
use objc2::rc::*;

use objc2_app_kit::{NSResponder, NSView};
use objc2_foundation::{NSObject, MainThreadMarker};
use objc2_quartz_core::{CAMetalLayer};

declare_class!(
    ///
    /// FloDrawView is an NSView subclass used as the main view for a flo_draw implementation.
    /// It adds support for pressure-sensitive events and sets transistion times to 0 for rendering.
    ///
    #[derive(Debug)]
    pub struct FloDrawView;

    unsafe impl ClassType for FloDrawView {
        #[inherits(NSResponder, NSObject)]
        type Super                  = NSView;
        type Mutability             = mutability::MainThreadOnly;
        const NAME: &'static str    = "FloDrawView";
    }

    impl DeclaredClass for FloDrawView {
        type Ivars = ();
    }

    unsafe impl FloDrawView {

    }
);

impl FloDrawView {
    ///
    /// Creates a new FloDrawView
    ///
    pub fn new() -> Retained<Self> {
        // Allocate the view
        let main_thread_marker      = MainThreadMarker::new().expect("Must be on main thread");
        let this                    = main_thread_marker.alloc().set_ivars(());
        let this: Retained<Self>    = unsafe { msg_send_id![super(this), init] };

        // Set up with a metal layer
        this.setWantsLayer(true);

        let metal_layer  = unsafe { CAMetalLayer::new() };
        unsafe { let _: () = msg_send![&this, setLayer: &*metal_layer]; }

        this
    }
}

use objc2::*;

use objc2_app_kit::{NSResponder, NSView};
use objc2_foundation::{NSObject};

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

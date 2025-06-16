use objc2::*;
use objc2::rc::*;

use objc2_app_kit::{NSColor, NSResponder, NSView};
use objc2_foundation::{NSObject, MainThreadMarker, NSSize};
use objc2_quartz_core::{CALayer, CAMetalLayer, CATransaction};

use winit::window::{Window};
use winit::raw_window_handle_05::{HasRawWindowHandle, RawWindowHandle};

use std::sync::*;

pub struct FloDrawViewVars {
    /// The layer that we should render on
    metal_layer: Retained<CAMetalLayer>,
}

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
        type Ivars = FloDrawViewVars;
    }

    unsafe impl FloDrawView {
        ///
        /// Resizes this view
        ///
        #[method(setFrameSize:)]
        fn set_frame_size(&self, newSize: NSSize) {
            // Perform the normal resizing request
            unsafe { let _: () = msg_send![super(self), setFrameSize: newSize]; }

            // Resize the rendering layer within this view
            // Disable the transaction so the resize doesn't do a silly animation
            CATransaction::begin();
            CATransaction::setAnimationDuration(0.0);
            CATransaction::disableActions();

            self.reposition_metal_layer();

            CATransaction::commit();
        }
    }
);

impl FloDrawView {
    ///
    /// Creates a new FloDrawView
    ///
    pub fn new() -> Retained<Self> {
        let main_thread_marker      = MainThreadMarker::new().expect("Must be on main thread");

        // Allocate a layer to use as our render target
        let metal_layer  = unsafe { CAMetalLayer::new() };

        let ivars = FloDrawViewVars {
            metal_layer: metal_layer.clone(),
        };

        // Allocate the view
        let this                    = main_thread_marker.alloc().set_ivars(ivars);
        let this: Retained<Self>    = unsafe { msg_send_id![super(this), init] };

        // Set up with a metal layer
        this.setWantsLayer(true);

        if let Some(layer) = unsafe { this.layer() } {
            //let cg_color: Id<NSObject> = unsafe { msg_send_id![&*NSColor::blueColor(), CGColor] };
            //unsafe { let _: () = msg_send![&*layer, setBackgroundColor: &*cg_color]; }

            //layer.addSublayer(&*metal_layer);
        }

        this.reposition_metal_layer();

        this
    }

    ///
    /// Retrieves the metal layer for this FloDrawView
    ///
    fn metal_layer(&self) -> Retained<CAMetalLayer> {
        self.ivars().metal_layer.clone()
    }

    ///
    /// Repositions the metal layer within this view
    ///
    fn reposition_metal_layer(&self) {
        // Fetch the layer
        let layer           = unsafe { self.layer() };
        let render_layer    = self.metal_layer();

        if let Some(layer) = layer {
            // Get the parent bounds (TODO: if we want to support scrolling, the visible rect of the parent too)
            let parent_bounds = layer.bounds();

            // Set the bounds of the render layer
            render_layer.setBounds(parent_bounds);
        }
    }

    ///
    /// Sets this view as the main view of the specified window
    ///
    pub fn attach_to(&self, window: &Arc<Window>) {
        if let RawWindowHandle::AppKit(appkit) = window.raw_window_handle() {
            // Fetch the root view from the window
            let root_view: Option<Retained<NSView>> = unsafe { Id::retain(appkit.ns_view.cast()) };
            let root_view                           = root_view.expect("Window must have a root view");

            // Add as a subview of the root view
            unsafe { root_view.addSubview(self); }

            // Size to fit
            unsafe { self.setFrame(root_view.bounds()); }

            self.reposition_metal_layer();
        } else {
            // We should be running on OS X here, so we should get an appkit window
            panic!("Was expecting an appkit window");
        }
    }
}

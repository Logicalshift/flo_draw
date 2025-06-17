use crate::wgpu::winit_window::*;

use objc2::*;
use objc2::rc::{Id, Retained};
use objc2::runtime::{ProtocolObject};

use objc2_app_kit::{NSAutoresizingMaskOptions, NSResponder, NSView};
use objc2_foundation::{NSObject, MainThreadMarker, NSSize, NSNull, NSDictionary, NSString, ns_string, NSCopying};
use objc2_quartz_core::{CAAction, CAMetalLayer, CATransaction};

use winit::raw_window_handle_05::{HasRawWindowHandle, RawWindowHandle};
use wgpu;

use std::sync::*;

pub struct FloDrawViewVars {
    /// The layer that we should render on
    metal_layer: Retained<CAMetalLayer>,

    /// The winit window that is being rendered in this view
    window: Option<Weak<Mutex<WinitWindow>>>,
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
        type Ivars = Mutex<FloDrawViewVars>;
    }

    unsafe impl FloDrawView {
        ///
        /// Resizes this view
        ///
        #[method(setFrameSize:)]
        fn set_frame_size(&self, new_size: NSSize) {
            // Perform the normal resizing request
            unsafe { let _: () = msg_send![super(self), setFrameSize: new_size]; }

            // Resize the rendering layer within this view
            // Disable the transaction so the resize doesn't do a silly animation
            CATransaction::begin();
            CATransaction::setAnimationDuration(0.0);
            CATransaction::setDisableActions(true);

            self.reposition_metal_layer();

            if let Some(window) = self.window() {
                if let Ok(mut window) = window.try_lock() {
                    window.redraw_immediate(new_size.width as _, new_size.height as _);
                    println!("Redraw immediate");
                } else {
                    println!("Window locked");
                }
            }

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
            metal_layer:    metal_layer.clone(),
            window:         None,
        };

        // Allocate the view
        let this                    = main_thread_marker.alloc().set_ivars(Mutex::new(ivars));
        let this: Retained<Self>    = unsafe { msg_send_id![super(this), init] };

        // Set up with a metal layer
        this.setWantsLayer(true);

        if let Some(layer) = unsafe { this.layer() } {
            layer.addSublayer(&*metal_layer);
            unsafe { 
                let sublayers = ns_string!("sublayers").copy();
                let contents  = ns_string!("content").copy();

                layer.setActions(Some(&*NSDictionary::<NSString, ProtocolObject<dyn CAAction>>::dictionaryWithObject_forKey(
                    &*Retained::cast(NSNull::null()), &*Retained::cast(sublayers.clone())))); 
                layer.setActions(Some(&*NSDictionary::<NSString, ProtocolObject<dyn CAAction>>::dictionaryWithObject_forKey(
                    &*Retained::cast(NSNull::null()), &*Retained::cast(contents.clone())))); 

                metal_layer.setActions(Some(&*NSDictionary::<NSString, ProtocolObject<dyn CAAction>>::dictionaryWithObject_forKey(
                    &*Retained::cast(NSNull::null()), &*Retained::cast(sublayers)))); 
                metal_layer.setActions(Some(&*NSDictionary::<NSString, ProtocolObject<dyn CAAction>>::dictionaryWithObject_forKey(
                    &*Retained::cast(NSNull::null()), &*Retained::cast(contents)))); 
            }
        }

        this.reposition_metal_layer();

        this
    }

    ///
    /// Retrieves the metal layer for this FloDrawView
    ///
    fn metal_layer(&self) -> Retained<CAMetalLayer> {
        self.ivars().lock().unwrap().metal_layer.clone()
    }

    ///
    /// Retrieves the window that this view is attached to
    ///
    fn window(&self) -> Option<Arc<Mutex<WinitWindow>>> {
        self.ivars().lock().unwrap().window.as_ref().and_then(|window| window.upgrade())
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
            render_layer.setFrame(parent_bounds);
        }
    }

    ///
    /// Sets this view as the main view of the specified window
    ///
    pub fn attach_to(&self, window: &Arc<Mutex<WinitWindow>>) {
        self.ivars().lock().unwrap().window = Some(Arc::downgrade(window));

        let window = window.lock().unwrap();

        if let Some(RawWindowHandle::AppKit(appkit)) = window.window().map(|window| window.raw_window_handle()) {
            // Fetch the root view from the window
            let root_view: Option<Retained<NSView>> = unsafe { Id::retain(appkit.ns_view.cast()) };
            let root_view                           = root_view.expect("Window must have a root view");

            // Add as a subview of the root view
            unsafe { root_view.addSubview(self); }

            // Size to fit
            unsafe { self.setFrame(root_view.bounds()); }

            // Set the root view to resize its subviews
            unsafe { root_view.setAutoresizesSubviews(true); }
            unsafe { self.setAutoresizingMask(NSAutoresizingMaskOptions::NSViewWidthSizable.union(NSAutoresizingMaskOptions::NSViewHeightSizable)); }

            self.reposition_metal_layer();

            if let Some(layer) = unsafe { root_view.layer() } {
                unsafe { 
                    let sublayers = ns_string!("sublayers").copy();
                    let contents  = ns_string!("content").copy();

                    layer.setActions(Some(&*NSDictionary::<NSString, ProtocolObject<dyn CAAction>>::dictionaryWithObject_forKey(
                        &*Retained::cast(NSNull::null()), &*Retained::cast(sublayers.clone())))); 
                    layer.setActions(Some(&*NSDictionary::<NSString, ProtocolObject<dyn CAAction>>::dictionaryWithObject_forKey(
                        &*Retained::cast(NSNull::null()), &*Retained::cast(contents.clone())))); 
                }
            }

            if let Some(layer) = unsafe { self.layer() } {
                let metal_layer = self.metal_layer();

                unsafe { 
                    let sublayers = ns_string!("sublayers").copy();
                    let contents  = ns_string!("content").copy();

                    layer.setActions(Some(&*NSDictionary::<NSString, ProtocolObject<dyn CAAction>>::dictionaryWithObject_forKey(
                        &*Retained::cast(NSNull::null()), &*Retained::cast(sublayers.clone())))); 
                    layer.setActions(Some(&*NSDictionary::<NSString, ProtocolObject<dyn CAAction>>::dictionaryWithObject_forKey(
                        &*Retained::cast(NSNull::null()), &*Retained::cast(contents.clone())))); 

                    metal_layer.setActions(Some(&*NSDictionary::<NSString, ProtocolObject<dyn CAAction>>::dictionaryWithObject_forKey(
                        &*Retained::cast(NSNull::null()), &*Retained::cast(sublayers)))); 
                    metal_layer.setActions(Some(&*NSDictionary::<NSString, ProtocolObject<dyn CAAction>>::dictionaryWithObject_forKey(
                        &*Retained::cast(NSNull::null()), &*Retained::cast(contents)))); 
                }
            }
        } else {
            // We should be running on OS X here, so we should get an appkit window
            panic!("Was expecting an appkit window");
        }
    }

    ///
    /// Creates a WGPU surface for this view
    ///
    pub fn create_surface<'a>(&self, instance: &wgpu::Instance) -> wgpu::Surface<'a> {
        let layer                           = self.metal_layer();
        let layer_ptr: *const CAMetalLayer  = Retained::as_ptr(&layer);
        let target                          = wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(layer_ptr as *mut _);

        unsafe { instance.create_surface_unsafe(target).unwrap() }
    }
}

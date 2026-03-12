use super::events::*;
use crate::events::*;
use crate::platform::winit::*;

#[cfg(feature="render-wgpu")] use crate::wgpu::winit_thread::*;
#[cfg(feature="render-wgpu")] use crate::wgpu::winit_thread_event::*;

#[cfg(all(not(feature="render-wgpu"), feature="render-software"))] use crate::software::*;

use objc2::*;
use objc2::rc::{Id, Retained};
#[cfg(feature="render-wgpu")] use objc2::runtime::{ProtocolObject};

use objc2_app_kit::{NSAutoresizingMaskOptions, NSResponder, NSView, NSEvent};
use objc2_foundation::{NSObject, MainThreadMarker};

#[cfg(feature="render-wgpu")] use objc2_foundation::{NSNull, NSDictionary, NSString, ns_string, NSCopying, NSSize};
#[cfg(feature="render-wgpu")] use objc2_quartz_core::{CAAction, CAMetalLayer, CATransaction};
#[cfg(feature="render-wgpu")] use wgpu;

use winit::window::{WindowId};
use winit::raw_window_handle_05::{HasRawWindowHandle, RawWindowHandle};

use std::sync::*;

pub struct FloDrawViewVars {
    /// The layer that we should render on
    #[cfg(feature="render-wgpu")] metal_layer: Retained<CAMetalLayer>,

    /// The winit window that is being rendered in this view
    window: Option<Arc<dyn PlatformWindow>>,

    /// The buttons that are currently held down
    buttons: Vec<Button>,
}

define_class!(
    ///
    /// FloDrawView is an NSView subclass used as the main view for a flo_draw implementation.
    /// It adds support for pressure-sensitive events and sets transistion times to 0 for rendering.
    ///
    #[unsafe(super(NSView, NSResponder, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "FloDrawView"]
    #[ivars = Mutex<FloDrawViewVars>]
    pub struct FloDrawView;

    impl FloDrawView {
        ///
        /// Resizes this view
        ///
        #[cfg(feature="render-wgpu")]
        #[unsafe(method(setFrameSize:))]
        fn set_frame_size(&self, new_size: NSSize) {
            // Perform the normal resizing request
            let _: () = unsafe { msg_send![super(self), setFrameSize: new_size] };

            // Resize the rendering layer within this view
            // Disable the transaction so the resize doesn't do a silly animation
            CATransaction::begin();
            CATransaction::setAnimationDuration(0.0);
            CATransaction::setDisableActions(true);

            self.reposition_metal_layer();

            // TODO: if we want to fully eliminate the 'glitching' that can occur here, we need to send the resize event now, then process events from the scene until it
            // becomes idle (or at least until the corresponding redraw request has gone through), before committing the transaction and returning to the main winit
            // event loop.

            CATransaction::commit();
        }

        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            self.send_window_event(draw_pointer_event_for_nsevent(self, PointerAction::Move, self.buttons(), event));
            self.send_window_event(draw_pointer_event_for_nsevent(self, PointerAction::ButtonDown, self.press_button(Button::Left), event));
        }

        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, event: &NSEvent) {
            self.send_window_event(draw_pointer_event_for_nsevent(self, PointerAction::Move, self.buttons(), event));
            self.send_window_event(draw_pointer_event_for_nsevent(self, PointerAction::ButtonUp, self.release_button(Button::Left), event));
        }

        #[unsafe(method(rightMouseDown:))]
        fn right_mouse_down(&self, event: &NSEvent) {
            self.send_window_event(draw_pointer_event_for_nsevent(self, PointerAction::Move, self.buttons(), event));
            self.send_window_event(draw_pointer_event_for_nsevent(self, PointerAction::ButtonUp, self.press_button(Button::Right), event));
        }

        #[unsafe(method(rightMouseUp:))]
        fn right_mouse_up(&self, event: &NSEvent) {
            self.send_window_event(draw_pointer_event_for_nsevent(self, PointerAction::Move, self.buttons(), event));
            self.send_window_event(draw_pointer_event_for_nsevent(self, PointerAction::ButtonUp, self.release_button(Button::Right), event));
        }

        #[unsafe(method(otherMouseDown:))]
        fn other_mouse_down(&self, event: &NSEvent) {
            let button = match event.buttonNumber() {
                0 => Button::Left,
                1 => Button::Right,
                2 => Button::Middle,
                other => Button::Other(other as _),
            };

            self.send_window_event(draw_pointer_event_for_nsevent(self, PointerAction::Move, self.buttons(), event));
            self.send_window_event(draw_pointer_event_for_nsevent(self, PointerAction::ButtonDown, self.press_button(button), event));
        }

        #[unsafe(method(otherMouseUp:))]
        fn other_mouse_up(&self, event: &NSEvent) {
            let button = match event.buttonNumber() {
                0 => Button::Left,
                1 => Button::Right,
                2 => Button::Middle,
                other => Button::Other(other as _),
            };

            self.send_window_event(draw_pointer_event_for_nsevent(self, PointerAction::Move, self.buttons(), event));
            self.send_window_event(draw_pointer_event_for_nsevent(self, PointerAction::ButtonUp, self.release_button(button), event));
        }

        #[unsafe(method(mouseMoved:))]
        fn mouse_moved(&self, event: &NSEvent) {
            self.send_window_event(draw_pointer_event_for_nsevent(self, PointerAction::Move, self.buttons(), event));
        }

        #[unsafe(method(mouseDragged:))]
        fn mouse_dragged(&self, event: &NSEvent) {
            self.send_window_event(draw_pointer_event_for_nsevent(self, PointerAction::Move, self.buttons(), event));
        }

        #[unsafe(method(rightMouseDragged:))]
        fn right_mouse_dragged(&self, event: &NSEvent) {
            self.send_window_event(draw_pointer_event_for_nsevent(self, PointerAction::Move, self.buttons(), event));
        }

        #[unsafe(method(otherMouseDragged:))]
        fn other_mouse_dragged(&self, event: &NSEvent) {
            self.send_window_event(draw_pointer_event_for_nsevent(self, PointerAction::Move, self.buttons(), event));
        }

        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, _event: &NSEvent) {
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
        }
    }
);

impl FloDrawView {
    ///
    /// Creates a new FloDrawView
    ///
    #[cfg(feature="render-wgpu")]
    pub fn new() -> Retained<Self> {
        let main_thread_marker = MainThreadMarker::new().expect("Must be on main thread");

        // Allocate a layer to use as our render target
        let metal_layer = CAMetalLayer::new();

        let ivars = FloDrawViewVars {
            metal_layer:    metal_layer.clone(),
            window:         None,
            buttons:        vec![],
        };

        // Allocate the view
        let this                    = main_thread_marker.alloc().set_ivars(Mutex::new(ivars));
        let this: Retained<Self>    = unsafe { msg_send_id![super(this), init] };

        // Set up with a metal layer
        this.setWantsLayer(true);

        if let Some(layer) = this.layer() {
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
    /// Creates a new FloDrawView
    ///
    #[cfg(all(not(feature="render-wgpu"), feature="render-software"))]
    pub fn new() -> Retained<Self> {
        let main_thread_marker = MainThreadMarker::new().expect("Must be on main thread");

        let ivars = FloDrawViewVars {
            window:     None,
            buttons:    vec![],
        };

        // Allocate the view
        let this                    = main_thread_marker.alloc().set_ivars(Mutex::new(ivars));
        let this: Retained<Self>    = unsafe { msg_send_id![super(this), init] };

        this
    }

    ///
    /// Retrieves the metal layer for this FloDrawView
    ///
    #[cfg(feature="render-wgpu")]
    fn metal_layer(&self) -> Retained<CAMetalLayer> {
        self.ivars().lock().unwrap().metal_layer.clone()
    }

    ///
    /// Repositions the metal layer within this view
    ///
    #[cfg(feature="render-wgpu")]
    fn reposition_metal_layer(&self) {
        // Fetch the layer
        let layer           = self.layer();
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
    #[cfg(feature="render-wgpu")]
    pub fn attach_to(&self, window: &Arc<dyn PlatformWindow>) {
        self.ivars().lock().unwrap().window = Some(Arc::clone(&window));

        if let Some(RawWindowHandle::AppKit(appkit)) = window.window().map(|window| window.raw_window_handle()) {
            // Fetch the root view from the window
            let root_view: Option<Retained<NSView>> = unsafe { Id::retain(appkit.ns_view.cast()) };
            let root_view                           = root_view.expect("Window must have a root view");

            // Add as a subview of the root view
            root_view.addSubview(self);

            // Size to fit
            self.setFrame(root_view.bounds());

            // Set the root view to resize its subviews
            root_view.setAutoresizesSubviews(true);
            self.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable.union(NSAutoresizingMaskOptions::ViewHeightSizable));

            self.reposition_metal_layer();

            if let Some(layer) = root_view.layer() {
                unsafe { 
                    let sublayers = ns_string!("sublayers").copy();
                    let contents  = ns_string!("content").copy();

                    layer.setActions(Some(&*NSDictionary::<NSString, ProtocolObject<dyn CAAction>>::dictionaryWithObject_forKey(
                        &*Retained::cast(NSNull::null()), &*Retained::cast(sublayers.clone())))); 
                    layer.setActions(Some(&*NSDictionary::<NSString, ProtocolObject<dyn CAAction>>::dictionaryWithObject_forKey(
                        &*Retained::cast(NSNull::null()), &*Retained::cast(contents.clone())))); 
                }
            }

            if let Some(layer) = self.layer() {
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
    /// Sets this view as the main view of the specified window
    ///
    #[cfg(all(not(feature="render-wgpu"), feature="render-software"))] 
    pub fn attach_to(&self, window: &Arc<dyn PlatformWindow>) {
        self.ivars().lock().unwrap().window = Some(Arc::clone(&window));

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
            unsafe { self.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable.union(NSAutoresizingMaskOptions::ViewHeightSizable)); }
        } else {
            // We should be running on OS X here, so we should get an appkit window
            panic!("Was expecting an appkit window");
        }
    }

    ///
    /// Returns the current set of pressed buttons in this view
    ///
    #[inline]
    fn buttons(&self) -> Vec<Button> {
        let ivars = self.ivars().lock().unwrap();

        ivars.buttons.clone()
    }

    ///
    /// Sets a button as pressed (returning the updated button state)
    ///
    #[inline]
    fn press_button(&self, pressed_button: Button) -> Vec<Button> {
        let mut ivars = self.ivars().lock().unwrap();

        ivars.buttons.retain(|existing_button| existing_button != &pressed_button);
        ivars.buttons.push(pressed_button);

        ivars.buttons.clone()
    }

    ///
    /// Sets a button as released (returning the updated button state)
    ///
    #[inline]
    fn release_button(&self, released_button: Button) -> Vec<Button> {
        let mut ivars = self.ivars().lock().unwrap();

        ivars.buttons.retain(|existing_button| existing_button != &released_button);

        ivars.buttons.clone()
    }

    ///
    /// Creates a WGPU surface for this view
    ///
    #[cfg(feature="render-wgpu")]
    pub fn create_surface<'a>(&self, instance: &wgpu::Instance) -> wgpu::Surface<'a> {
        let layer                           = self.metal_layer();
        let layer_ptr: *const CAMetalLayer  = Retained::as_ptr(&layer);
        let target                          = wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(layer_ptr as *mut _);

        unsafe { instance.create_surface_unsafe(target).unwrap() }
    }

    ///
    /// Retrieves the window ID for the window that contains this view
    ///
    pub fn window_id(&self) -> Option<WindowId> {
        // Fetch the window if it's available
        let window  = self.ivars().lock().unwrap().window.clone()?;
        let window  = window.window()?;

        Some(window.id())
    }

    ///
    /// Sends an event to the window that contains this view
    ///
    pub fn send_window_event(&self, event: DrawEvent) {
        if let Some(window_id) = self.window_id() {
            winit_thread().send_event(WinitThreadEvent::SendDrawEventToWindow(window_id, event));
        }
    }
}

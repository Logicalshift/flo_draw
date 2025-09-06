use flo_binding::*;
use flo_canvas_events::*;

// TODO: the 'feedback' properties like the actual size could be better implemented in a separate class, but it's not clear how to implement that cleanly without a big redesign

///
/// Trait implemented by objects that can provide properties for creating/updating a flo_draw window
///
/// Window properties are supplied as bindings to make it possible to update them after the window has
/// been created.
///
pub trait FloWindowProperties {
    ///
    /// The title of the window
    ///
    fn title(&self) -> BindRef<String>;

    ///
    /// The requested size of the window (before user resizing)
    ///
    /// The window will resize when this value changes, but won't bind to this value if the user resizes the widow.
    ///
    fn requested_size(&self) -> BindRef<(u64, u64)>;

    ///
    /// Set to true if the window should be fullscreen
    ///
    fn fullscreen(&self) -> BindRef<bool>;

    ///
    /// Set to true if the window should have decorations
    ///
    fn has_decorations(&self) -> BindRef<bool>;

    ///
    /// The mouse pointer to show for a window
    ///
    fn mouse_pointer(&self) -> BindRef<MousePointer>;

    ///
    /// The bounds of the viewport to the canvas to render within a window
    ///
    fn viewport_bounds(&self) -> BindRef<ViewportBounds>;

    ///
    /// A binding that specifies the actual size of the window in pixels
    ///
    /// If this is not None, this is updated when the window size changes. That is, this
    /// property is used to communicate information about the window back to the routine
    /// that created it. Most code should only call `get()` on this binding. Setting this
    /// value does not update the window size on screen (the `requested_size` binding is
    /// used for that)
    ///
    /// This is often used with the viewport bounds property, for example, this will
    /// force a 1-1 pixel relationship:
    ///
    /// ```
    /// # let window_properties = WindowProperties::from(());
    /// let actual_size = window_properties.actual_size().unwrap();
    ///
    /// window_properties.viewport_bounds = compute(move || ViewportBounds::FitExact((0.0, 0.0), actual_sized.get()));
    /// ```
    ///
    fn actual_size(&self) -> Option<Binding<(f32, f32)>>;

    ///
    /// A binding that specifies the scaling factor for the window.
    ///
    /// If this is not None, this is updated when the window scale changes. Ie, code should
    /// almost always be calling 'get()' on this value and not setting it. Setting this value
    /// does not change the scale of the window; this is only for feedback from the window program
    ///
    fn actual_scale(&self) -> Option<Binding<f32>>;
}

///
/// '()' can be used to create a window with the default title
///
impl FloWindowProperties for () {
    fn title(&self) -> BindRef<String>                      { BindRef::from(bind("flo_draw".to_string())) }
    fn requested_size(&self) -> BindRef<(u64, u64)>         { BindRef::from(bind((1024, 768))) }
    fn fullscreen(&self) -> BindRef<bool>                   { BindRef::from(bind(false)) }
    fn has_decorations(&self) -> BindRef<bool>              { BindRef::from(bind(true)) }
    fn mouse_pointer(&self) -> BindRef<MousePointer>        { BindRef::from(bind(MousePointer::SystemDefault)) }
    fn viewport_bounds(&self) -> BindRef<ViewportBounds>    { BindRef::from(bind(ViewportBounds::default())) }

    fn actual_size(&self) -> Option<Binding<(f32, f32)>>    { None }
    fn actual_scale(&self) -> Option<Binding<f32>>          { None }
}

///
/// A string can be used to set just the window title
///
impl<'a> FloWindowProperties for &'a str {
    fn title(&self) -> BindRef<String>                      { BindRef::from(bind(self.to_string())) }
    fn requested_size(&self) -> BindRef<(u64, u64)>         { BindRef::from(bind((1024, 768))) }
    fn fullscreen(&self) -> BindRef<bool>                   { BindRef::from(bind(false)) }
    fn has_decorations(&self) -> BindRef<bool>              { BindRef::from(bind(true)) }
    fn mouse_pointer(&self) -> BindRef<MousePointer>        { BindRef::from(bind(MousePointer::SystemDefault)) }
    fn viewport_bounds(&self) -> BindRef<ViewportBounds>    { BindRef::from(bind(ViewportBounds::default())) }

    fn actual_size(&self) -> Option<Binding<(f32, f32)>>    { None }
    fn actual_scale(&self) -> Option<Binding<f32>>          { None }
}

///
/// The window properties struct provides a copy of all of the bindings for a window, and is a good way to provide
/// custom bindings (for example, if you want to be able to toggle the window betwen fullscreen and a normal display)
///
#[derive(Clone)]
pub struct WindowProperties {
    pub title:              BindRef<String>,
    pub requested_size:     BindRef<(u64, u64)>,
    pub fullscreen:         BindRef<bool>,
    pub has_decorations:    BindRef<bool>,
    pub mouse_pointer:      BindRef<MousePointer>,
    pub viewport_bounds:    BindRef<ViewportBounds>,

    actual_size:            Binding<(f32, f32)>,
    actual_scale:           Binding<f32>,
}

#[inline]
fn u64_to_f32(val: (u64, u64)) -> (f32, f32) {
    (val.0 as _, val.1 as _)
}


impl WindowProperties {
    ///
    /// Creates a clone of an object implementing the FloWindowProperties trait
    ///
    pub fn from<T: FloWindowProperties>(properties: &T) -> WindowProperties {
        WindowProperties {
            title:              properties.title(),
            requested_size:     properties.requested_size(),
            fullscreen:         properties.fullscreen(),
            has_decorations:    properties.has_decorations(),
            mouse_pointer:      properties.mouse_pointer(),
            viewport_bounds:    properties.viewport_bounds(),

            actual_size:        properties.actual_size().unwrap_or_else(|| bind(u64_to_f32(properties.requested_size().get()))),
            actual_scale:       properties.actual_scale().unwrap_or_else(|| bind(1.0)),
        }
    }
}

impl FloWindowProperties for WindowProperties {
    fn title(&self) -> BindRef<String>                      { self.title.clone() }
    fn requested_size(&self) -> BindRef<(u64, u64)>         { self.requested_size.clone() }
    fn fullscreen(&self) -> BindRef<bool>                   { self.fullscreen.clone() }
    fn has_decorations(&self) -> BindRef<bool>              { self.has_decorations.clone() }
    fn mouse_pointer(&self) -> BindRef<MousePointer>        { self.mouse_pointer.clone() }
    fn viewport_bounds(&self) -> BindRef<ViewportBounds>    { self.viewport_bounds.clone() }

    fn actual_size(&self) -> Option<Binding<(f32, f32)>>    { Some(self.actual_size.clone()) }
    fn actual_scale(&self) -> Option<Binding<f32>>          { Some(self.actual_scale.clone()) }
}

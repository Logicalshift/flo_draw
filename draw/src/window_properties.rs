use flo_binding::*;
use flo_canvas_events::*;

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
    /// The initial size of the window
    ///
    fn size(&self) -> BindRef<(u64, u64)>;

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
}

///
/// '()' can be used to create a window with the default title
///
impl FloWindowProperties for () {
    fn title(&self) -> BindRef<String>                  { BindRef::from(bind("flo_draw".to_string())) }
    fn size(&self) -> BindRef<(u64, u64)>               { BindRef::from(bind((1024, 768))) }
    fn fullscreen(&self) -> BindRef<bool>               { BindRef::from(bind(false)) }
    fn has_decorations(&self) -> BindRef<bool>          { BindRef::from(bind(true)) }
    fn mouse_pointer(&self) -> BindRef<MousePointer>    { BindRef::from(bind(MousePointer::SystemDefault)) }
}

///
/// A string can be used to set just the window title
///
impl<'a> FloWindowProperties for &'a str {
    fn title(&self) -> BindRef<String>                  { BindRef::from(bind(self.to_string())) }
    fn size(&self) -> BindRef<(u64, u64)>               { BindRef::from(bind((1024, 768))) }
    fn fullscreen(&self) -> BindRef<bool>               { BindRef::from(bind(false)) }
    fn has_decorations(&self) -> BindRef<bool>          { BindRef::from(bind(true)) }
    fn mouse_pointer(&self) -> BindRef<MousePointer>    { BindRef::from(bind(MousePointer::SystemDefault)) }
}

///
/// The window properties struct provides a copy of all of the bindings for a window, and is a good way to provide
/// custom bindings (for example, if you want to be able to toggle the window betwen fullscreen and a normal display)
///
#[derive(Clone)]
pub struct WindowProperties {
    pub title:              BindRef<String>,
    pub size:               BindRef<(u64, u64)>,
    pub fullscreen:         BindRef<bool>,
    pub has_decorations:    BindRef<bool>,
    pub mouse_pointer:      BindRef<MousePointer>
}

impl WindowProperties {
    ///
    /// Creates a clone of an object implementing the FloWindowProperties trait
    ///
    pub fn from<T: FloWindowProperties>(properties: &T) -> WindowProperties {
        WindowProperties {
            title:              properties.title(),
            size:               properties.size(),
            fullscreen:         properties.fullscreen(),
            has_decorations:    properties.has_decorations(),
            mouse_pointer:      properties.mouse_pointer()
        }
    }
}

impl FloWindowProperties for WindowProperties {
    fn title(&self) -> BindRef<String>                  { self.title.clone() }
    fn size(&self) -> BindRef<(u64, u64)>               { self.size.clone() }
    fn fullscreen(&self) -> BindRef<bool>               { self.fullscreen.clone() }
    fn has_decorations(&self) -> BindRef<bool>          { self.has_decorations.clone() }
    fn mouse_pointer(&self) -> BindRef<MousePointer>    { self.mouse_pointer.clone() }
}

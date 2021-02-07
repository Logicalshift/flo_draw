///
/// The button on a mouse or other device
///
/// If a device only has one means of input (eg, a pen being pressed against the screen),
/// this is considered to be the 'Left' button.
///
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Button {
    Left,
    Middle,
    Right,
    Other(u64)
}

///
/// The action associated with a pointer event
///
pub enum PointerAction {
    /// Moving a pointer with no buttons pressed
    Move,

    /// A new button has been pressed
    ButtonDown,

    /// Moving the pointer with a button pressed
    Drag,

    /// A button has been released
    ButtonUp,

    /// A button has been released in a cancellation gesture (eg, due to palm rejection), invalidating a previous drag action
    Cancel
}

///
/// Describes the state of a pointer device
///
#[derive(Clone, Debug)]
pub struct PointerState {
    /// The x and y coordinates of the pointer's location in the window
    pub location_in_window: (f64, f64),

    /// If the view is displaying scaled content, this is the location of the pointer in the coordinate scheme of that content
    pub location_in_canvas: Option<(f64, f64)>,

    /// The buttons that are currently pressed down
    pub buttons: Vec<Button>,

    /// If the pointer device supports pressure, the pressure the user is applying (from 0.0 to 1.0)
    pub pressure: Option<f64>,

    /// tilt in degrees relative to the normal to the surface of the screen along the X and Y axes (values from -90 to 90)
    pub tilt: Option<(f64, f64)>
}

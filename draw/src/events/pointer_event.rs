///
/// A unique identifier assigned to a specific pointer on the system (a device that has a mouse and touch input might be tracking
/// multiple pointer devices)
///
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct PointerId(u64);

///
/// The button on a mouse or other device
///
/// If a device only has one means of input (eg, a pen being pressed against the screen),
/// this is considered to be the 'Left' button.
///
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Button {
    Left,
    Middle,
    Right,
    Other(u64)
}

///
/// The action associated with a pointer event
///
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PointerAction {
    /// The pointer has entered the window
    Enter,

    /// The pointer has left the window
    Leave,

    /// Moving a pointer with no buttons pressed
    Move,

    /// A new button has been pressed
    ButtonDown,

    /// Moving the pointer with a button pressed (drag events can move outside the bounds of the window)
    Drag,

    /// A button has been released
    ButtonUp,

    /// A button has been released in a cancellation gesture (eg, due to palm rejection), invalidating a previous drag action
    Cancel
}

///
/// Describes the state of a pointer device
///
/// Note: while we support the various different axes that a tablet device might support, presently glutin does not provide
/// this information to us, so these values are currently always set to 'None'.
///
#[derive(Clone, PartialEq, Debug)]
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
    pub tilt: Option<(f64, f64)>,

    /// If the device supports detecting rotation around its own axis, this is amount of rotation in degrees (values from -180 to 180)
    pub rotation: Option<f64>,

    /// If the device has a 'flow rate' adjustment (emulating an airbrush, for example) this is the value of that (from 0.0 to 1.0).
    pub flow_rate: Option<f64>
}

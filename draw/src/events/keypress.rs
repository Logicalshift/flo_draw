///
/// Represents a keypress
///
#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug)]
pub enum KeyPress {
    ModifierShift,
    ModifierCtrl,
    ModifierAlt,
    ModifierMeta,
    ModifierSuper,
    ModifierHyper,

    KeyTab,

    KeyA,
    KeyB,
    KeyC,
    KeyD,
    KeyE,
    KeyF,
    KeyG,
    KeyH,
    KeyI,
    KeyJ,
    KeyK,
    KeyL,
    KeyM,
    KeyN,
    KeyO,
    KeyP,
    KeyQ,
    KeyR,
    KeyS,
    KeyT,
    KeyU,
    KeyV,
    KeyW,
    KeyX,
    KeyY,
    KeyZ,

    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,
    Key0,

    KeyUp,
    KeyDown,
    KeyLeft,
    KeyRight,

    KeyBackslash,
    KeyForwardslash,
    KeyBacktick,
    KeyComma,
    KeyFullstop,
    KeySemicolon,
    KeyQuote,
    KeyMinus,
    KeyEquals,

    KeyEscape,
    KeyInsert,
    KeyHome,
    KeyPgUp,
    KeyDelete,
    KeyEnd,
    KeyPgDown,
    KeyBackspace,
    KeyEnter,

    KeyF1,
    KeyF2,
    KeyF3,
    KeyF4,
    KeyF5,
    KeyF6,
    KeyF7,
    KeyF8,
    KeyF9,
    KeyF10,
    KeyF11,
    KeyF12,
    KeyF13,
    KeyF14,
    KeyF15,
    KeyF16,

    KeyNumpad0,
    KeyNumpad1,
    KeyNumpad2,
    KeyNumpad3,
    KeyNumpad4,
    KeyNumpad5,
    KeyNumpad6,
    KeyNumpad7,
    KeyNumpad8,
    KeyNumpad9,
    KeyNumpadDivide,
    KeyNumpadMultiply,
    KeyNumpadMinus,
    KeyNumpadAdd,
    KeyNumpadEnter,
    KeyNumpadDecimal,
}

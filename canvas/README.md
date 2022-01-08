```toml
flo_canvas = "0.4"
```

# flo_canvas

`flo_canvas` is a library that provides a way to describe 2D drawings, without providing any
concrete implementation of how those drawings should be rendered. It supports streaming updates
to allow canvases to be displayed in any user interface library that understands the `Draw`
instructions, and it provides a serialization and deserialization mechanism for sending canvas
instructions to other applications.

This library was designed to support FlowBetween, an interactive animation editor. However,
it has several implementations that make it useful outside that context. In particular, the
`flo_draw` crate provides a straightforward way to render canvases into a window. `flo_render`
and `flo_render_canvas` combine to provide a general-purpose way of rendering 2D canvases using
modern 3D-accellerated graphics hardware: this includes the ability to render canvases 
off-screen to a bitmap on Linux, OS X and Windows systems.

FlowBetween itself has some implementations that are not quite so accessible but may still be 
of interest. In particular `canvas.js` provides an implementation of `flo_canvas` in javascript,
suitable for rendering to a HTML canvas.

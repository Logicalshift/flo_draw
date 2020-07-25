use super::stroke_settings::*;

use flo_render as render;
use flo_canvas as canvas;

use lyon::path;
use lyon::math::{Point};
use lyon::tessellation;
use lyon::tessellation::{VertexBuffers, BuffersBuilder, FillOptions, FillAttributes};

///
/// The tessellator turns canvas drawing instructions into triangle lists for rendering
///
pub struct Tessellator {
    /// The builder for the current path
    path_builder: Option<path::Builder>,

    /// None, or the built path
    current_path: Option<path::Path>,

    /// The rendered fill for this path
    fill: Option<VertexBuffers<render::Vertex2D, u16>>,

    /// The rendered stroke for this path
    stroke: Option<VertexBuffers<render::Vertex2D, u16>>,

    /// The path stroke settings for this path
    stroke_settings: StrokeSettings,

    /// The fill colour for this path
    fill_color: render::Rgba8
}

impl Tessellator {
    ///
    /// Creates a new path tessellator
    ///
    pub fn new() -> Tessellator {
        Tessellator {
            path_builder:       None,
            current_path:       None,
            fill:               None,
            stroke:             None,
            stroke_settings:    StrokeSettings::new(),
            fill_color:         render::Rgba8([0,0,0,255])
        }
    }

    ///
    /// Clears this tessellator
    ///
    pub fn clear(&mut self) {
        self.path_builder   = None;
        self.current_path   = None;
        self.fill           = None;
        self.stroke         = None;
    }

    ///
    /// Creates a builder by reading and replaying a path
    ///
    fn builder_from_path(path: &path::Path) -> path::Builder {
        let mut builder = path::Builder::new();

        for event in path.iter() {
            use path::Event::*;

            match event {
                Begin { at }                        => { builder.move_to(at); }
                Line { from: _, to }                => { builder.line_to(to); },
                Quadratic { from: _, ctrl, to }     => { builder.quadratic_bezier_to(ctrl, to); },
                Cubic { from: _, ctrl1, ctrl2, to } => { builder.cubic_bezier_to(ctrl1, ctrl2, to); },
                End { last: _, first: _, close }    => { if close { builder.close(); } }
            }
        }

        builder
    }

    ///
    /// Creates or retrieves the current path builder
    ///
    fn build<'a>(&'a mut self) -> &'a mut path::Builder {
        if self.path_builder.is_some() {
            // Already building a path: use the existing path builder
            self.path_builder.as_mut().unwrap()

        } else if let Some(path) = &self.current_path {
            // Amending a path that was built earlier: populate the builder from the existing path
            let builder         = Self::builder_from_path(path);
            self.path_builder   = Some(builder);
            self.current_path   = None;

            self.path_builder.as_mut().unwrap()

        } else {
            // Empty: create a new path builder and return that
            self.path_builder   = Some(path::Builder::new());

            self.path_builder.as_mut().unwrap()
        }
    }

    ///
    /// Changes a colour component to a u8 format
    ///
    fn col_to_u8(component: f32) -> u8 {
        if component > 1.0 {
            255
        } else if component < 0.0 {
            0
        } else {
            (component * 255.0) as u8
        }
    }

    ///
    /// Sets the fill colour for this tessellator
    ///
    pub fn set_fill_color(&mut self, color: &canvas::Color) {
        let (r, g, b, a)    = color.to_rgba_components();
        let (r, g, b, a)    = (Self::col_to_u8(r), Self::col_to_u8(g), Self::col_to_u8(b), Self::col_to_u8(a));

        self.fill_color     = render::Rgba8([r, g, b, a]);
    }

    ///
    /// Creates a fill using the current path (consuming the path builder, if there is one)
    ///
    pub fn fill(&mut self) {
        // Create the path from the path builder if there is one
        if let Some(path_builder) = self.path_builder.take() {
            self.current_path = Some(path_builder.build());
        }

        // Clear the current fill if there is one
        self.fill = None;

        if let Some(current_path) = &self.current_path {
            // Create the tessellator
            let mut tessellator         = tessellation::FillTessellator::new();
            let mut geometry            = VertexBuffers::new();
            let render::Rgba8(color)    = self.fill_color;  

            // Generate the path data into the geometry
            tessellator.tessellate_path(current_path, &FillOptions::default(),
                &mut BuffersBuilder::new(&mut geometry, move |point: Point, _attr: FillAttributes| {
                    render::Vertex2D {
                        pos:        point.to_array(),
                        tex_coord:  [0.0, 0.0],
                        color:      color
                    }
                })).unwrap();

            // Store the generated geometry
            self.fill = Some(geometry);
        }
    }
}

use crate::fill_state::*;

use lyon::path;
use lyon::math::{point};

///
/// The path that is being prepared for rendering
///
pub (super) struct PathState {
    pub (super) current_path:   Option<path::Path>,
    pub (super) in_subpath:     bool,
    pub (super) path_builder:   Option<path::path::Builder>,

    pub (super) fill_state:     FillState,
    pub (super) dash_pattern:   Vec<f32>,
}

impl Default for PathState {
    fn default() -> PathState {
        PathState {
            current_path:   None,
            in_subpath:     false,
            path_builder:   None,
            fill_state:     FillState::None,
            dash_pattern:   vec![],
        }
    }
}

impl PathState {
    /// Takes the current path builder and fills in the current_path from it
    #[inline]
    pub (super) fn build(&mut self) {
        if let Some(mut path_builder) = self.path_builder.take() {
            if self.in_subpath { path_builder.end(false); }
            self.current_path = Some(path_builder.build());
        }
    }

    /// Begins a new path
    #[inline]
    pub (super) fn tes_new_path(&mut self) {
        self.current_path   = None;
        self.in_subpath     = false;
        self.path_builder   = Some(path::Path::builder());
    }

    /// Move to a new point
    #[inline]
    pub (super) fn tes_move(&mut self, x: f32, y: f32) {
        if self.in_subpath {
            self.path_builder.as_mut().map(|builder| builder.end(false));
        }
        self.path_builder.get_or_insert_with(|| path::Path::builder())
            .begin(point(x, y));
        self.in_subpath = true;
    }

    /// Line to point
    #[inline]
    pub (super) fn tes_line(&mut self, x: f32, y: f32) {
        if self.in_subpath {
            self.path_builder.get_or_insert_with(|| path::Path::builder())
                .line_to(point(x, y));
        } else {
            self.path_builder.get_or_insert_with(|| path::Path::builder())
                .begin(point(x, y));
            self.in_subpath = true;
        }
    }

    /// Bezier curve to point
    #[inline]
    pub (super) fn tes_bezier_curve(&mut self, (cp1x, cp1y): (f32, f32), (cp2x, cp2y): (f32, f32), (px, py): (f32, f32)) {
        if self.in_subpath {
            self.path_builder.get_or_insert_with(|| path::Path::builder())
                .cubic_bezier_to(point(cp1x, cp1y), point(cp2x, cp2y), point(px, py));
        } else {
            self.path_builder.get_or_insert_with(|| path::Path::builder())
                .begin(point(px, py));
            self.in_subpath = true;
        }
    }

    /// Closes the current path
    #[inline]
    pub (super) fn tes_close_path(&mut self) {
        self.path_builder.get_or_insert_with(|| path::Path::builder())
            .end(true);
        self.in_subpath = false;
    }
}

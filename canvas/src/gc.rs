use super::draw::*;
use super::color::*;
use super::transform2d::*;

use curves::*;
use curves::arc;
use curves::bezier::BezierCurve;

///
/// A graphics context provides the basic set of graphics actions that can be performed 
///
pub trait GraphicsContext {
    fn new_path(&mut self);
    fn move_to(&mut self, x: f32, y: f32);
    fn line_to(&mut self, x: f32, y: f32);
    fn bezier_curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32);
    fn close_path(&mut self);
    fn fill(&mut self);
    fn stroke(&mut self);
    fn line_width(&mut self, width: f32);
    fn line_width_pixels(&mut self, width: f32);
    fn line_join(&mut self, join: LineJoin);
    fn line_cap(&mut self, cap: LineCap);
    fn new_dash_pattern(&mut self);
    fn dash_length(&mut self, length: f32);
    fn dash_offset(&mut self, offset: f32);
    fn fill_color(&mut self, col: Color);
    fn stroke_color(&mut self, col: Color);
    fn blend_mode(&mut self, mode: BlendMode);
    fn identity_transform(&mut self);
    fn canvas_height(&mut self, height: f32);
    fn center_region(&mut self, minx: f32, miny: f32, maxx: f32, maxy: f32);
    fn transform(&mut self, transform: Transform2D);
    fn unclip(&mut self);
    fn clip(&mut self);
    fn store(&mut self);
    fn restore(&mut self);
    fn free_stored_buffer(&mut self);
    fn push_state(&mut self);
    fn pop_state(&mut self);
    fn clear_canvas(&mut self);
    fn layer(&mut self, layer_id: u32);
    fn layer_blend(&mut self, layer_id: u32, blend_mode: BlendMode);
    fn clear_layer(&mut self);

    fn draw(&mut self, d: Draw) {
        use self::Draw::*;

        match d {
            NewPath                                     => self.new_path(),
            Move(x, y)                                  => self.move_to(x, y),
            Line(x, y)                                  => self.line_to(x, y),
            BezierCurve((x1, y1), (x2, y2), (x3, y3))   => self.bezier_curve_to(x1, y1, x2, y2, x3, y3),
            ClosePath                                   => self.close_path(),
            Fill                                        => self.fill(),
            Stroke                                      => self.stroke(),
            LineWidth(width)                            => self.line_width(width),
            LineWidthPixels(width)                      => self.line_width_pixels(width),
            LineJoin(join)                              => self.line_join(join),
            LineCap(cap)                                => self.line_cap(cap),
            NewDashPattern                              => self.new_dash_pattern(),
            DashLength(dash_length)                     => self.dash_length(dash_length),
            DashOffset(dash_offset)                     => self.dash_offset(dash_offset),
            FillColor(col)                              => self.fill_color(col),
            StrokeColor(col)                            => self.stroke_color(col),
            BlendMode(blendmode)                        => self.blend_mode(blendmode),
            IdentityTransform                           => self.identity_transform(),
            CanvasHeight(height)                        => self.canvas_height(height),
            CenterRegion((minx, miny), (maxx, maxy))    => self.center_region(minx, miny, maxx, maxy),
            MultiplyTransform(transform)                => self.transform(transform),
            Unclip                                      => self.unclip(),
            Clip                                        => self.clip(),
            Store                                       => self.store(),
            Restore                                     => self.restore(),
            FreeStoredBuffer                            => self.free_stored_buffer(),
            PushState                                   => self.push_state(),
            PopState                                    => self.pop_state(),
            ClearCanvas                                 => self.clear_canvas(),
            Layer(layer_id)                             => self.layer(layer_id),
            LayerBlend(layer_id, blend_mode)            => self.layer_blend(layer_id, blend_mode),
            ClearLayer                                  => self.clear_layer()
        }
    }
}

///
/// GraphicsPrimitives adds new primitives that can be built directly from a graphics context
///
pub trait GraphicsPrimitives : GraphicsContext {
    ///
    /// Draws a rectangle between particular coordinates
    /// 
    fn rect(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) {
        self.move_to(x1, y1);
        self.line_to(x1, y2);
        self.line_to(x2, y2);
        self.line_to(x2, y1);
        self.line_to(x1, y1);
        self.close_path();
    }

    ///
    /// Draws a circle at a particular point
    /// 
    fn circle(&mut self, center_x: f32, center_y: f32, radius: f32) {
        // Generate the circle and turn it into bezier curves
        let circle                      = arc::Circle::new(Coord2(center_x as f64, center_y as f64), radius as f64);
        let curves: Vec<bezier::Curve>  = circle.to_curves();

        // Move to the start point
        let start_point = curves[0].start_point();
        self.move_to(start_point.x() as f32, start_point.y() as f32);

        // Draw the curves
        for c in curves {
            gc_draw_bezier(self, &c);
        }

        self.close_path();
    }
}

///
/// Draws the specified bezier curve in a graphics context (assuming we're already at the start position) 
///
pub fn gc_draw_bezier<Gc: GraphicsContext+?Sized, Coord: Coordinate2D+Coordinate, Curve: BezierCurve<Point=Coord>>(gc: &mut Gc, curve: &Curve) {
    let end         = curve.end_point();
    let (cp1, cp2)  = curve.control_points();

    gc.bezier_curve_to(end.x() as f32, end.y() as f32, cp1.x() as f32, cp1.y() as f32, cp2.x() as f32, cp2.y() as f32);
}


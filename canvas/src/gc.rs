use super::draw::*;
use super::color::*;
use super::transform2d::*;

use curves::*;
use curves::arc;
use curves::bezier::BezierCurve;

use std::iter;

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
    fn winding_rule(&mut self, winding_rule: WindingRule);
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
    fn clear_canvas(&mut self, color: Color);
    fn layer(&mut self, layer_id: u32);
    fn layer_blend(&mut self, layer_id: u32, blend_mode: BlendMode);
    fn clear_layer(&mut self);
    fn sprite(&mut self, sprite_id: SpriteId);
    fn clear_sprite(&mut self);
    fn sprite_transform(&mut self, transform: SpriteTransform);
    fn draw_sprite(&mut self, sprite_id: SpriteId);

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
            WindingRule(rule)                           => self.winding_rule(rule),
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
            ClearCanvas(color)                          => self.clear_canvas(color),
            Layer(layer_id)                             => self.layer(layer_id),
            LayerBlend(layer_id, blend_mode)            => self.layer_blend(layer_id, blend_mode),
            ClearLayer                                  => self.clear_layer(),
            Sprite(sprite_id)                           => self.sprite(sprite_id),
            ClearSprite                                 => self.clear_sprite(),
            SpriteTransform(transform)                  => self.sprite_transform(transform),
            DrawSprite(sprite_id)                       => self.draw_sprite(sprite_id)
        }
    }

    fn draw_list<'a>(&'a mut self, drawing: Box<dyn 'a+Iterator<Item=Draw>>) {
        for d in drawing {
            self.draw(d);
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
        for d in draw_rect(x1, y1, x2, y2) {
            self.draw(d);
        }
    }

    ///
    /// Draws a circle at a particular point
    ///
    fn circle(&mut self, center_x: f32, center_y: f32, radius: f32) {
        for d in draw_circle(center_x, center_y, radius) {
            self.draw(d);
        }
    }
}

///
/// Returns the drawing commands for a rectangle
///
pub fn draw_rect(x1: f32, y1: f32, x2: f32, y2: f32) -> Vec<Draw> {
    use self::Draw::*;

    vec![
        Move(x1, y1),
        Line(x1, y2),
        Line(x2, y2),
        Line(x2, y1),
        Line(x1, y1),
        ClosePath
    ]
}

///
/// Returns the drawing commands for a circle
///
pub fn draw_circle(center_x: f32, center_y: f32, radius: f32) -> Vec<Draw> {
    use self::Draw::*;

    // Generate the circle and turn it into bezier curves
    let circle                          = arc::Circle::new(Coord2(center_x as f64, center_y as f64), radius as f64);
    let curves: Vec<bezier::Curve<_>>   = circle.to_curves();
    let start_point                     = curves[0].start_point();

    // Draw the curves
    let curves  = curves.into_iter().map(|curve| Draw::from(&curve));

    // Complete the path
    let path    = iter::once(Move(start_point.x() as f32, start_point.y() as f32))
        .chain(curves)
        .chain(iter::once(ClosePath));

    path.collect()
}

impl<'a, Curve: BezierCurve> From<&'a Curve> for Draw
where Curve::Point: Coordinate2D {
    fn from(curve: &'a Curve) -> Draw {
        let end         = curve.end_point();
        let (cp1, cp2)  = curve.control_points();

        Draw::BezierCurve(
            (end.x() as f32, end.y() as f32),
            (cp1.x() as f32, cp1.y() as f32),
            (cp2.x() as f32, cp2.y() as f32))
    }
}

///
/// Draws the specified bezier curve in a graphics context (assuming we're already at the start position)
///
pub fn gc_draw_bezier<Gc: GraphicsContext+?Sized, Curve: BezierCurve>(gc: &mut Gc, curve: &Curve)
where Curve::Point: Coordinate2D {
    gc.draw(Draw::from(curve))
}

///
/// A Vec<Draw> can be treated as a target for graphics primitives (just pushing the appropriate draw instructions)
///
impl GraphicsContext for Vec<Draw> {
    #[inline] fn new_path(&mut self)                                                    { self.push(Draw::NewPath); }
    #[inline] fn move_to(&mut self, x: f32, y: f32)                                     { self.push(Draw::Move(x, y)); }
    #[inline] fn line_to(&mut self, x: f32, y: f32)                                     { self.push(Draw::Line(x, y)); }
    #[inline] fn bezier_curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32) { self.push(Draw::BezierCurve((x1, y1), (x2, y2), (x3, y3))); }
    #[inline] fn close_path(&mut self)                                                  { self.push(Draw::ClosePath); }
    #[inline] fn fill(&mut self)                                                        { self.push(Draw::Fill); }
    #[inline] fn stroke(&mut self)                                                      { self.push(Draw::Stroke); }
    #[inline] fn line_width(&mut self, width: f32)                                      { self.push(Draw::LineWidth(width)); }
    #[inline] fn line_width_pixels(&mut self, width: f32)                               { self.push(Draw::LineWidthPixels(width)); }
    #[inline] fn line_join(&mut self, join: LineJoin)                                   { self.push(Draw::LineJoin(join)); }
    #[inline] fn line_cap(&mut self, cap: LineCap)                                      { self.push(Draw::LineCap(cap)); }
    #[inline] fn winding_rule(&mut self, rule: WindingRule)                             { self.push(Draw::WindingRule(rule)); }
    #[inline] fn new_dash_pattern(&mut self)                                            { self.push(Draw::NewDashPattern); }
    #[inline] fn dash_length(&mut self, length: f32)                                    { self.push(Draw::DashLength(length)); }
    #[inline] fn dash_offset(&mut self, offset: f32)                                    { self.push(Draw::DashOffset(offset)); }
    #[inline] fn fill_color(&mut self, col: Color)                                      { self.push(Draw::FillColor(col)); }
    #[inline] fn stroke_color(&mut self, col: Color)                                    { self.push(Draw::StrokeColor(col)); }
    #[inline] fn blend_mode(&mut self, mode: BlendMode)                                 { self.push(Draw::BlendMode(mode)); }
    #[inline] fn identity_transform(&mut self)                                          { self.push(Draw::IdentityTransform); }
    #[inline] fn canvas_height(&mut self, height: f32)                                  { self.push(Draw::CanvasHeight(height)); }
    #[inline] fn center_region(&mut self, minx: f32, miny: f32, maxx: f32, maxy: f32)   { self.push(Draw::CenterRegion((minx, miny), (maxx, maxy))); }
    #[inline] fn transform(&mut self, transform: Transform2D)                           { self.push(Draw::MultiplyTransform(transform)); }
    #[inline] fn unclip(&mut self)                                                      { self.push(Draw::Unclip); }
    #[inline] fn clip(&mut self)                                                        { self.push(Draw::Clip); }
    #[inline] fn store(&mut self)                                                       { self.push(Draw::Store); }
    #[inline] fn restore(&mut self)                                                     { self.push(Draw::Restore); }
    #[inline] fn free_stored_buffer(&mut self)                                          { self.push(Draw::FreeStoredBuffer); }
    #[inline] fn push_state(&mut self)                                                  { self.push(Draw::PushState); }
    #[inline] fn pop_state(&mut self)                                                   { self.push(Draw::PopState); }
    #[inline] fn clear_canvas(&mut self, color: Color)                                  { self.push(Draw::ClearCanvas(color)); }
    #[inline] fn layer(&mut self, layer_id: u32)                                        { self.push(Draw::Layer(layer_id)); }
    #[inline] fn layer_blend(&mut self, layer_id: u32, blend_mode: BlendMode)           { self.push(Draw::LayerBlend(layer_id, blend_mode)); }
    #[inline] fn clear_layer(&mut self)                                                 { self.push(Draw::ClearLayer); }
    #[inline] fn sprite(&mut self, sprite_id: SpriteId)                                 { self.push(Draw::Sprite(sprite_id)); }
    #[inline] fn clear_sprite(&mut self)                                                { self.push(Draw::ClearSprite); }
    #[inline] fn sprite_transform(&mut self, transform: SpriteTransform)                { self.push(Draw::SpriteTransform(transform)); }
    #[inline] fn draw_sprite(&mut self, sprite_id: SpriteId)                            { self.push(Draw::DrawSprite(sprite_id)); }

    #[inline]
    fn draw(&mut self, d: Draw) {
        self.push(d);
    }

    #[inline]
    fn draw_list<'b>(&'b mut self, drawing: Box<dyn 'b+Iterator<Item=Draw>>) {
        self.extend(drawing)
    }
}

///
/// A Vec<Draw> can be treated as a target for graphics primitives (just pushing the appropriate draw instructions)
///
impl GraphicsPrimitives for Vec<Draw> {

}

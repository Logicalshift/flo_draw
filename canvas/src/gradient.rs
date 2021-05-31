use crate::color::*;

use itertools::*;

use std::cmp::{Ordering};

///
/// Identifies a gradient
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GradientId(pub u64);

///
/// Operations that can be applied to a gradient
///
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GradientOp {
    /// Clears the gradient and starts a new one with the given initial colour
    New(Color),

    /// Adds a new gradient stop of the specified colour
    AddStop(f32, Color)
}

///
/// Converts a f32 value between 0 and 1 to a byte
///
#[inline]
fn colour_component_to_byte(component: f32) -> u8 {
    if component < 0.0 { 
        0 
    } else if component > 1.0 {
        255
    } else {
        (component * 255.0) as u8
    }
}

///
/// Converts a floating point quad to a set of RGBA bytes
///
#[inline]
fn components_to_bytes(components: (f32, f32, f32, f32)) -> [u8; 4] {
    [
        colour_component_to_byte(components.0),
        colour_component_to_byte(components.1),
        colour_component_to_byte(components.2),
        colour_component_to_byte(components.3)
    ]
}

///
/// Creates a gradient scale, as 8-bit RGBA quads from a set of gradient operations
///
pub fn gradient_scale<GradientIter: IntoIterator<Item=GradientOp>, const N: usize>(description: GradientIter) -> [[u8; 4]; N] {
    // Create a blank scale
    let mut scale = [[0, 0, 0, 0]; N];

    // Create a list of colour stops by position
    let mut stops = description.into_iter()
        .map(|op| match op {
            GradientOp::New(col)            => (0.0, col.to_rgba_components()),
            GradientOp::AddStop(pos, col)   => (pos, col.to_rgba_components())
        })
        .collect::<Vec<_>>();

    // Order by position
    stops.sort_by(|(pos_a, _), (pos_b, _)| pos_a.partial_cmp(pos_b).unwrap_or(Ordering::Equal));

    if stops.len() == 0 {
        // No stops means we return the blank scale
        scale
    } else if stops.len() == 1 {
        // A single stop just uses that as a flat colour
        [components_to_bytes(stops[0].1); N]
    } else {
        // Fill the scale using the stops
        let min_pos             = stops[0].0 as f64;
        let max_pos             = stops[stops.len()-1].0 as f64;

        debug_assert!(max_pos > min_pos);

        let distance_per_step   = (max_pos - min_pos) / ((N-1) as f64);
        let final_color         = components_to_bytes(stops[stops.len()-1].1);
        let mut idx             = 0;
        let mut stop_iter       = stops.into_iter().tuple_windows();
        let mut current_stop    = stop_iter.next().unwrap();

        while idx < (N-1) {
            let pos             = ((idx as f64) * distance_per_step) + min_pos;

            // Get the current position
            let ((start_pos, (r1, g1, b1, a1)), (end_pos, (r2, g2, b2, a2))) = &current_stop;

            let start_pos       = *start_pos as f64;
            let end_pos         = *end_pos as f64;

            // Move to the next stop if the current position is already past the end
            if pos >= end_pos {
                current_stop = stop_iter.next().unwrap();
                continue;
            }

            // Blend the colour between the end position and the start position
            let ratio           = ((pos-start_pos)/(end_pos-start_pos)) as f32;
            let (r, g, b, a)    = (
                (r2-r1)*ratio + r1,
                (g2-g1)*ratio + g1,
                (b2-b1)*ratio + b1,
                (a2-a1)*ratio + a1
            );

            // Write this component to the current index
            scale[idx]  = components_to_bytes((r, g, b, a));

            // Move to the next position before continuing
            idx         += 1;
        }

        debug_assert!(idx == N-1);
        scale[idx] = final_color;

        scale
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn generate_basic_gradient_scale() {
        let scale = gradient_scale::<_, 16>(vec![
            GradientOp::New(Color::Rgba(0.0, 0.0, 0.0, 0.0)), 
            GradientOp::AddStop(1.0, Color::Rgba(1.0, 1.0, 1.0, 1.0))
        ]);

        assert!(scale[0]  == [0, 0, 0, 0]);
        assert!(scale[15] == [255, 255, 255, 255]);

        assert!(scale[1]  == [17, 17, 17, 17]);
        assert!(scale[2]  == [34, 34, 34, 34]);
        assert!(scale[3]  == [51, 51, 51, 51]);
        assert!(scale[4]  == [68, 68, 68, 68]);
        assert!(scale[5]  == [85, 85, 85, 85]);
        assert!(scale[6]  == [102, 102, 102, 102]);
        assert!(scale[7]  == [119, 119, 119, 119]);
        assert!(scale[8]  == [136, 136, 136, 136]);
        assert!(scale[9]  == [153, 153, 153, 153]);
        assert!(scale[10] == [170, 170, 170, 170]);
        assert!(scale[11] == [187, 187, 187, 187]);
        assert!(scale[12] == [204, 204, 204, 204]);
        assert!(scale[13] == [221, 221, 221, 221]);
        assert!(scale[14] == [238, 238, 238, 238]);
    }

    #[test]
    fn generate_basic_gradient_scale_1024() {
        let scale = gradient_scale::<_, 1024>(vec![
            GradientOp::New(Color::Rgba(0.0, 0.0, 0.0, 0.0)), 
            GradientOp::AddStop(1.0, Color::Rgba(1.0, 1.0, 1.0, 1.0))
        ]);

        assert!(scale[0]  == [0, 0, 0, 0]);
        assert!(scale[1023] == [255, 255, 255, 255]);
    }

    #[test]
    fn scale_basic_red() {
        let scale = gradient_scale::<_, 16>(vec![
            GradientOp::New(Color::Rgba(0.0, 0.0, 0.0, 0.0)), 
            GradientOp::AddStop(1.0, Color::Rgba(1.0, 0.0, 0.0, 0.0))
        ]);

        assert!(scale[0]  == [0, 0, 0, 0]);
        assert!(scale[15] == [255, 0, 0, 0]);

        assert!(scale[1]  == [17, 0, 0, 0]);
        assert!(scale[2]  == [34, 0, 0, 0]);
        assert!(scale[3]  == [51, 0, 0, 0]);
        assert!(scale[4]  == [68, 0, 0, 0]);
        assert!(scale[5]  == [85, 0, 0, 0]);
        assert!(scale[6]  == [102, 0, 0, 0]);
        assert!(scale[7]  == [119, 0, 0, 0]);
        assert!(scale[8]  == [136, 0, 0, 0]);
        assert!(scale[9]  == [153, 0, 0, 0]);
        assert!(scale[10] == [170, 0, 0, 0]);
        assert!(scale[11] == [187, 0, 0, 0]);
        assert!(scale[12] == [204, 0, 0, 0]);
        assert!(scale[13] == [221, 0, 0, 0]);
        assert!(scale[14] == [238, 0, 0, 0]);
    }

    #[test]
    fn scale_basic_green() {
        let scale = gradient_scale::<_, 16>(vec![
            GradientOp::New(Color::Rgba(0.0, 0.0, 0.0, 0.0)), 
            GradientOp::AddStop(1.0, Color::Rgba(0.0, 1.0, 0.0, 0.0))
        ]);

        assert!(scale[0]  == [0, 0, 0, 0]);
        assert!(scale[15] == [0, 255, 0, 0]);

        assert!(scale[1]  == [0, 17, 0, 0]);
        assert!(scale[2]  == [0, 34, 0, 0]);
        assert!(scale[3]  == [0, 51, 0, 0]);
        assert!(scale[4]  == [0, 68, 0, 0]);
        assert!(scale[5]  == [0, 85, 0, 0]);
        assert!(scale[6]  == [0, 102, 0, 0]);
        assert!(scale[7]  == [0, 119, 0, 0]);
        assert!(scale[8]  == [0, 136, 0, 0]);
        assert!(scale[9]  == [0, 153, 0, 0]);
        assert!(scale[10] == [0, 170, 0, 0]);
        assert!(scale[11] == [0, 187, 0, 0]);
        assert!(scale[12] == [0, 204, 0, 0]);
        assert!(scale[13] == [0, 221, 0, 0]);
        assert!(scale[14] == [0, 238, 0, 0]);
    }

    #[test]
    fn scale_basic_blue() {
        let scale = gradient_scale::<_, 16>(vec![
            GradientOp::New(Color::Rgba(0.0, 0.0, 0.0, 0.0)), 
            GradientOp::AddStop(1.0, Color::Rgba(0.0, 0.0, 1.0, 0.0))
        ]);

        assert!(scale[0]  == [0, 0, 0, 0]);
        assert!(scale[15] == [0, 0, 255, 0]);

        assert!(scale[1]  == [0, 0, 17, 0]);
        assert!(scale[2]  == [0, 0, 34, 0]);
        assert!(scale[3]  == [0, 0, 51, 0]);
        assert!(scale[4]  == [0, 0, 68, 0]);
        assert!(scale[5]  == [0, 0, 85, 0]);
        assert!(scale[6]  == [0, 0, 102, 0]);
        assert!(scale[7]  == [0, 0, 119, 0]);
        assert!(scale[8]  == [0, 0, 136, 0]);
        assert!(scale[9]  == [0, 0, 153, 0]);
        assert!(scale[10] == [0, 0, 170, 0]);
        assert!(scale[11] == [0, 0, 187, 0]);
        assert!(scale[12] == [0, 0, 204, 0]);
        assert!(scale[13] == [0, 0, 221, 0]);
        assert!(scale[14] == [0, 0, 238, 0]);
    }

    #[test]
    fn scale_basic_alpha() {
        let scale = gradient_scale::<_, 16>(vec![
            GradientOp::New(Color::Rgba(0.0, 0.0, 0.0, 0.0)), 
            GradientOp::AddStop(1.0, Color::Rgba(0.0, 0.0, 0.0, 1.0))
        ]);

        assert!(scale[0]  == [0, 0, 0, 0]);
        assert!(scale[15] == [0, 0, 0, 255]);

        assert!(scale[1]  == [0, 0, 0, 17]);
        assert!(scale[2]  == [0, 0, 0, 34]);
        assert!(scale[3]  == [0, 0, 0, 51]);
        assert!(scale[4]  == [0, 0, 0, 68]);
        assert!(scale[5]  == [0, 0, 0, 85]);
        assert!(scale[6]  == [0, 0, 0, 102]);
        assert!(scale[7]  == [0, 0, 0, 119]);
        assert!(scale[8]  == [0, 0, 0, 136]);
        assert!(scale[9]  == [0, 0, 0, 153]);
        assert!(scale[10] == [0, 0, 0, 170]);
        assert!(scale[11] == [0, 0, 0, 187]);
        assert!(scale[12] == [0, 0, 0, 204]);
        assert!(scale[13] == [0, 0, 0, 221]);
        assert!(scale[14] == [0, 0, 0, 238]);
    }

    #[test]
    fn generate_two_stop_scale() {
        let scale = gradient_scale::<_, 17>(vec![
            GradientOp::New(Color::Rgba(0.0, 0.0, 0.0, 1.0)), 
            GradientOp::AddStop(0.5, Color::Rgba(1.0, 1.0, 1.0, 1.0)),
            GradientOp::AddStop(1.0, Color::Rgba(0.0, 0.0, 0.0, 1.0))
        ]);

        for x in 0..17 {
            if x < 8 {
                let p = (255.0 / 8.0) * (x as f32);
                let p = p as u8;

                assert!(scale[x] == [p, p, p, 255]);
            } else if x > 8 {
                let p = (255.0 / 8.0) * ((16-x) as f32);
                let p = p as u8;

                assert!(scale[x] == [p, p, p, 255]);
            } else {
                assert!(scale[x] == [255, 255, 255, 255]);
            }
        }
    }
}

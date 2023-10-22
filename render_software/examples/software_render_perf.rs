//!
//! Measures the performance of various stages of the software rendering process
//!
//! One of the design aims of flo_draw is to make all the components fully accessible while keeping the
//! overall API simple. An advantage of this approach is that it's possible to easily perform individual
//! rendering stages if needed, which is quite handy for this kind of performance tool.
//!
//! We use our own way of measuring perf rather than criterion here too, this is because criterion can be
//! quite slow with a lot of items in it, and also can't be customised to show things like 'frames per
//! second' like we do here.
//!

use flo_render_software::edgeplan::EdgeDescriptor;
use flo_render_software::pixel::*;
use flo_render_software::edges::*;
use flo_render_software::edgeplan::*;
use flo_canvas::curves::arc::*;
use flo_canvas::curves::geo::*;

use smallvec::*;

use std::time::{Instant, Duration};

struct TimingResult {
    /// The number of times the function was called
    iterations: usize,

    /// The total time taken to call the specified number of iterations
    total_time: Duration,

    /// The time for each call (seconds)
    time_per_call: f64,

    /// The number of calls that can be made in 1 frame (~16ms)
    calls_per_frame: f64,
}

///
/// Formats a value in seconds for display
///
fn format_seconds(seconds: f64) -> String {
    if seconds < 1e-6 {
        format!("{:.1}ns", seconds * 1e9)
    } else if seconds < 1e-3 {
        format!("{:.1}µs", seconds * 1e6)
    } else if seconds < 1.0 {
        format!("{:.1}ms", seconds * 1e3)
    } else if seconds < 60.0 {
        format!("{:.1}s", seconds)
    } else {
        format!("{:.0}m{:.1}s", (seconds/60.0).floor(), seconds%60.0)
    }
}

impl TimingResult {
    ///
    /// Creates a summary of this timing result
    ///
    pub fn summary(&self) -> String {
        format!("{} calls made in {}. {} per call, {:.1} calls per frame",
            self.iterations, 
            format_seconds((self.total_time.as_nanos() as f64) / 1e9),
            format_seconds(self.time_per_call),
            self.calls_per_frame)
    }

    ///
    /// Creates a summary of this timing result with an FPS value
    ///
    pub fn summary_fps(&self) -> String {
        format!("{} calls made in {}. {} per call, {:.1} calls per frame ({:.1} fps)",
            self.iterations, 
            format_seconds((self.total_time.as_nanos() as f64) / 1e9),
            format_seconds(self.time_per_call),
            self.calls_per_frame,
            1.0 / self.time_per_call)
    }
}

///
/// Calls a function `iterations` times and times it
///
fn time<T>(iterations: usize, action: impl FnMut() -> T) -> TimingResult {
    use std::hint::{black_box};

    let mut action = action;

    // Warm up
    for _ in 0..100 {
        black_box(action());
    }

    // Measure the time from now
    let start_time = Instant::now();

    // Perform the action repeatedly
    for _ in 0..iterations {
        black_box(action());
    }

    // Convert the time to 
    let total_time      = Instant::now().duration_since(start_time);
    let total_seconds   = (total_time.as_nanos() as f64) / 1_000_000_000.0;

    TimingResult {
        iterations:         iterations,
        total_time:         total_time,
        time_per_call:      total_seconds / (iterations as f64),
        calls_per_frame:    (1.0/60.0) / (total_seconds / (iterations as f64)),
    }
}

fn print_header(name: &str) {
    println!("\n\x1b[1m{}\x1b[22m", name);
}

fn main() {
    use std::hint::{black_box};

    // Simple pixel fill
    print_header("Basic pixel fill");

    let mut frame   = vec![U8RgbaPremultipliedPixel::from_components([0, 0, 0, 0]); 1920 * 1080];
    let val         = U8RgbaPremultipliedPixel::from_components([12, 13, 14, 15]);

    let simple_fill_u8_frame = time(1_000, || {
        for pix in frame.iter_mut() {
            *pix = val;
        }

        black_box(&mut frame);
    });
    let simple_fill_u8_frame_checked = time(1_000, || {
        for idx in 0..frame.len() {
            frame[idx] = val;
        }

        black_box(&mut frame);
    });
    let simple_fill_u8_frame_unchecked = time(1_000, || {
        for idx in 0..frame.len() {
            unsafe { *frame.get_unchecked_mut(idx) = val; }
        }

        black_box(&mut frame);
    });
    let mut f32_pix = vec![F32LinearPixel::from_components([0.5, 0.5, 0.5, 1.0]); 1920];
    let simple_fill = time(100_000, || {
        for idx in 0..(f32_pix.len()) {
            f32_pix[idx] = F32LinearPixel::from_components([0.1, 0.2, 0.3, 0.4]);
        }
        black_box(&mut f32_pix);
    });
    let simple_fill_unchecked = time(100_000, || {
        for idx in 0..(f32_pix.len()) {
            unsafe {
                *f32_pix.get_unchecked_mut(idx) = F32LinearPixel::from_components([0.1, 0.2, 0.3, 0.4]);
            }
        }
        black_box(&mut f32_pix);
    });
    let simple_fill_frame = time(1_000, || {
        for _ in 0..1080 {
            for idx in 0..(f32_pix.len()) {
                f32_pix[idx] = F32LinearPixel::from_components([0.1, 0.2, 0.3, 0.4]);
            }
            black_box(&mut f32_pix);
        }
    });
    let simple_fill_frame_unchecked = time(1_000, || {
        for _ in 0..1080 {
            for idx in 0..(f32_pix.len()) {
                unsafe {
                    *f32_pix.get_unchecked_mut(idx) = F32LinearPixel::from_components([0.1, 0.2, 0.3, 0.4]);
                }
            }
            black_box(&mut f32_pix);
        }
    });
    let simple_fill_frame_iterator = time(1_000, || {
        for _ in 0..1080 {
            for pix in f32_pix.iter_mut() {
                *pix = F32LinearPixel::from_components([0.1, 0.2, 0.3, 0.4]);
            }
            black_box(&mut f32_pix);
        }
    });
    println!("  U8 simple fill frame iterator: {}", simple_fill_u8_frame.summary_fps());
    println!("  U8 simple fill frame checked: {}", simple_fill_u8_frame_checked.summary_fps());
    println!("  U8 simple fill frame unchecked: {}", simple_fill_u8_frame_unchecked.summary_fps());
    println!("  F32 simple fill: {}", simple_fill.summary());
    println!("  F32 simple fill unchecked: {}", simple_fill_unchecked.summary());
    println!("  F32 simple fill frame: {}", simple_fill_frame.summary_fps());
    println!("  F32 simple fill frame unchecked: {}", simple_fill_frame_unchecked.summary_fps());
    println!("  F32 simple fill frame iterator: {}", simple_fill_frame_iterator.summary_fps());

    // Gamma correct from an f32 and an i32 buffer
    print_header("Gamma correct to generate output");

    let mut target_buf  = vec![U8RgbaPremultipliedPixel::default(); 1920];
    let target_buf      = &mut target_buf;

    let f32_pix             = vec![F32LinearPixel::from_components([0.5, 0.5, 0.5, 1.0]); 1920];
    let gamma_correct_f32   = time(100_000, || { F32LinearPixel::to_gamma_colorspace(&f32_pix, target_buf, 2.2); black_box(&target_buf); });
    let u32_pix             = vec![U32LinearPixel::from_components([32768u32.into(), 32768u32.into(), 32768u32.into(), 65535u32.into()]); 1920];
    let gamma_correct_i32   = time(100_000, || { U32LinearPixel::to_gamma_colorspace(&u32_pix, target_buf, 2.2); black_box(&target_buf); });

    let gamma_correct_f32_frame = time(1_000, || { for _ in 0..1080 { F32LinearPixel::to_gamma_colorspace(&f32_pix, target_buf, 2.2); black_box(&target_buf); } });

    println!("  F32 to_gamma_color_space: {}", gamma_correct_f32.summary());
    println!("  U32 to_gamma_color_space: {}", gamma_correct_i32.summary());
    println!("  F32 to_gamma_color_space whole frame: {}", gamma_correct_f32_frame.summary_fps());

    // Alpha blend
    print_header("Alpha blending");
    let mut f32_pix             = vec![F32LinearPixel::from_components([0.5, 0.5, 0.5, 1.0]); 1920];
    let blend_val               = F32LinearPixel::from_components([0.1, 0.2, 0.3, 0.4]);
    let alpha_blend_f32         = time(100_000, || { f32_pix.iter_mut().for_each(|pix| { black_box(pix.source_over(blend_val)); }); });
    let alpha_blend_f32_frame   = time(1_000, || { for _ in 0..1080 { f32_pix.iter_mut().for_each(|pix| { black_box(pix.source_over(blend_val)); }); } });
    println!("  F32 alpha blend line: {}", alpha_blend_f32.summary());
    println!("  F32 alpha blend whole frame: {}", alpha_blend_f32_frame.summary_fps());

    // Scan conversion
    print_header("Scan conversion");
    let circle      = Circle::new(Coord2(1920.0/2.0, 1080.0/2.0), 1080.0/2.0);
    let circle_path = circle.to_path::<BezierSubpath>();

    let prepare_as_bezier = time(10_000, || { black_box(circle.to_path::<BezierSubpath>().to_even_odd_edge(ShapeId::new()).prepare_to_render()) });
    let prepare_flattened = time(10_000, || { black_box(circle.to_path::<BezierSubpath>().to_flattened_non_zero_edge(ShapeId::new()).prepare_to_render()) });
    let flatten           = time(10_000, || { black_box(circle_path.clone().flatten_to_polyline(1.0, 0.25)); });

    println!("  Prepare to render bezier circle: {}", prepare_as_bezier.summary());
    println!("  Prepare to render flattened bezier (v high res): {}", prepare_flattened.summary());
    println!("  Flatten (pixel res): {}", flatten.summary());

    // Note that `to_flattened_even_odd_edge` assumes a coordinate scheme of -1 to 1 so it tends to generate much higher resolution images than are needed
    let mut circle_edge         = circle_path.clone().to_even_odd_edge(ShapeId::new());
    let mut circle_flattened    = circle_path.clone().to_flattened_even_odd_edge(ShapeId::new());
    let mut circle_polyline     = circle_path.clone().flatten_to_polyline(1.0, 0.25).to_even_odd_edge(ShapeId::new());
    let mut output              = vec![smallvec![]; 1080];
    circle_edge.prepare_to_render();
    circle_flattened.prepare_to_render();
    circle_polyline.prepare_to_render();

    let scan_convert_bezier = time(1_000, || { 
        circle_edge.intercepts(&(0..1080).map(|y_pos| y_pos as f64).collect::<Vec<_>>(), &mut output);
        black_box(&mut output);
    });
    let scan_convert_flattened = time(1_000, || { 
        circle_flattened.intercepts(&(0..1080).map(|y_pos| y_pos as f64).collect::<Vec<_>>(), &mut output);
        black_box(&mut output);
    });
    let scan_convert_polyline = time(1_000, || { 
        circle_polyline.intercepts(&(0..1080).map(|y_pos| y_pos as f64).collect::<Vec<_>>(), &mut output);
        black_box(&mut output);
    });

    println!("  Scan convert bezier circle: {}", scan_convert_bezier.summary_fps());
    println!("  Scan convert flattened circle (v high res): {}", scan_convert_flattened.summary_fps());
    println!("  Scan convert flattened circle (pixel res): {}", scan_convert_polyline.summary_fps());
}


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

use flo_render_software::pixel::*;

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

    print_header("Pixel functions");

    // Simple pixel fill
    let mut frame   = vec![U8RgbaPremultipliedPixel::from_components([0, 0, 0, 0]); 1920 * 1080];
    let val         = U8RgbaPremultipliedPixel::from_components([12, 13, 14, 15]);

    let simple_fill_u8_frame = time(1_000, || {
        for pix in frame.iter_mut() {
            *pix = val;
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
    let simple_fill_frame = time(1_000, || {
        for _ in 0..1080 {
            for idx in 0..(f32_pix.len()) {
                f32_pix[idx] = F32LinearPixel::from_components([0.1, 0.2, 0.3, 0.4]);
            }
        }
        black_box(&mut f32_pix);
    });
    println!("  U8 simple fill frame: {}", simple_fill_u8_frame.summary_fps());
    println!("  F32 simple fill: {}", simple_fill.summary());
    println!("  F32 simple fill frame: {}", simple_fill_frame.summary_fps());

    // Gamma correct from an f32 and an i32 buffer
    let mut target_buf  = vec![U8RgbaPremultipliedPixel::default(); 1920];
    let target_buf      = &mut target_buf;

    let f32_pix             = vec![F32LinearPixel::from_components([0.5, 0.5, 0.5, 1.0]); 1920];
    let gamma_correct_f32   = time(100_000, || { F32LinearPixel::to_gamma_colorspace(&f32_pix, target_buf, 2.2); black_box(&target_buf); });
    let u32_pix             = vec![U32LinearPixel::from_components([32768.into(), 32768.into(), 32768.into(), 65535.into()]); 1920];
    let gamma_correct_i32   = time(100_000, || { U32LinearPixel::to_gamma_colorspace(&u32_pix, target_buf, 2.2); black_box(&target_buf); });

    let gamma_correct_f32_frame = time(1_000, || { for _ in 0..1080 { F32LinearPixel::to_gamma_colorspace(&f32_pix, target_buf, 2.2); black_box(&target_buf); } });

    println!("  F32 to_gamma_color_space: {}", gamma_correct_f32.summary());
    println!("  U32 to_gamma_color_space: {}", gamma_correct_i32.summary());
    println!("  F32 to_gamma_color_space whole frame: {}", gamma_correct_f32_frame.summary_fps());

    let mut f32_pix             = vec![F32LinearPixel::from_components([0.5, 0.5, 0.5, 1.0]); 1920];
    let blend_val               = F32LinearPixel::from_components([0.1, 0.2, 0.3, 0.4]);
    let alpha_blend_f32         = time(100_000, || { f32_pix.iter_mut().for_each(|pix| { black_box(pix.source_over(blend_val)); }); });
    let alpha_blend_f32_frame   = time(1_000, || { for _ in 0..1080 { f32_pix.iter_mut().for_each(|pix| { black_box(pix.source_over(blend_val)); }); } });
    println!("  F32 alpha blend line: {}", alpha_blend_f32.summary());
    println!("  F32 alpha blend whole frame: {}", alpha_blend_f32_frame.summary_fps());
}


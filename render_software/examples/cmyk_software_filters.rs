use flo_render_software::canvas::*;
use flo_render_software::render::*;

use flo_render_software::draw::{CanvasDrawing, CanvasDrawingRegionRenderer};
use flo_render_software::pixel::{F32CmykaPixel, Pixel};
use flo_render_software::scanplan::ShardScanPlanner;
use std::f32;
use std::f32::consts::PI;
use std::fs::File;
use std::ops::{Add, Div, Mul};
use tiff::encoder::TiffEncoder;

///
/// Draws a bunch of circles in CMYK colorspace
///
pub fn main() {
	// Create drawing instructions for the png
	let mut canvas = vec![];
	
	let mut canvas_drawing = CanvasDrawing::<F32CmykaPixel, 5>::empty();
	canvas_drawing.draw(vec![Draw::ClearCanvas(Color::Cmyka(0.0, 0.0, 0.0, 0.0, 0.0))]);
	
	let alpha = 0.7;
	let spin = vec![
		(0.0, Color::Cmyka(1.0, 0.0, 0.0, 0.0, alpha)),
		(0.5, Color::Cmyka(0.0, 1.0, 0.0, 0.0, alpha)),
		(1.0, Color::Cmyka(0.0, 0.0, 1.0, 0.0, alpha)),
		(1.5, Color::Cmyka(0.0, 0.0, 0.0, 1.0, alpha)),
	];
	
	let mut sprite = vec![];
	sprite.sprite(SpriteId(1));
	for (rotate, color) in spin.iter() {
		let x = rotate.mul(PI).cos().mul(200.0);
		let y = rotate.mul(PI).sin().mul(200.0);
		sprite.new_path();
		sprite.circle(x, y, 250.0);
		sprite.fill_color(color.clone());
		sprite.fill();
	}
	sprite.layer(LayerId(0));
	
	// Clear the canvas and set up the coordinates
	canvas.canvas_height(1000.0);
	canvas.center_region(0.0, 0.0, 2000.0, 1000.0);
	
	canvas.layer(LayerId(0));
	canvas.clear_layer();
	
	canvas_drawing.draw(canvas);
	
	canvas_drawing.draw(sprite.iter().cloned());
	
	for part in 0..2 {
		let max_scale = 0.5;
		let min_scale = 0.1;
		let iters = 100;
		let div = iters as f32;
		let cycles = 1.66;
		let radius = 200.0;
		for i in 0..iters {
			let mut draw = vec![];
			draw.layer(LayerId(i + 1));
			let i = i as f32;
			let scale = max_scale - (max_scale - min_scale).mul(i.div(div).powi(2));
			let r = 200.0 + (radius / div) * i;
			let rot = PI.div(div).mul(cycles).mul(i).mul(2.0);
			let x = 500.0 + 1000.0 * part as f32 + rot.cos().mul(r);
			let y = 500.0 + rot.sin().mul(r);
			// println!("{scale} | {rot} | {r} | {x}, {y} ");
			draw.sprite_transform(SpriteTransform::Identity);
			draw.sprite_transform(SpriteTransform::Scale(scale, scale));
			draw.sprite_transform(SpriteTransform::Rotate(rot.mul(1.66).to_degrees()));
			draw.sprite_transform(SpriteTransform::Translate(x, y));
			match part {
				0 => draw.draw_sprite(SpriteId(1)),
				_ => draw.draw_sprite_with_filters(SpriteId(1), vec![TextureFilter::GaussianBlur(20.0 + 30.0 * (1.0 - i / div).powi(2))]), 
			};
			canvas_drawing.draw(draw);
		}
		
	}
	
	let width = 2000;
	let height = 1000;
	
	let renderer = CanvasDrawingRegionRenderer::new(
		ShardScanPlanner::default(), ScanlineRenderer::new(canvas_drawing.program_runner(height as _)),
		height
	);
	
	let renderer = F32CmykaFrameRenderer::new(renderer);
	let frame_size = GammaFrameSize { width, height, gamma: 2.2 };
	let mut pixel_data = vec![F32CmykaPixel::default(); width * height];
	
	renderer.render(&frame_size, &canvas_drawing, pixel_data.as_mut_slice());
    
    let file = File::create("cmyk_software_filters.tiff").unwrap();
    let mut tiff_enc = TiffEncoder::new_big(file).unwrap();
    let mut img_enc = tiff_enc.new_image::<tiff::encoder::colortype::CMYKA8>(width as u32, height as u32).unwrap();
	let es: &[u8] = &[2u8];
	img_enc.encoder().write_tag(tiff::tags::Tag::ExtraSamples, es).unwrap();
	
	let pixel_data = pixel_data.iter().map(|p| p.to_u8()).flatten().collect::<Vec<_>>();
    img_enc.write_data(&pixel_data).unwrap();
	
}

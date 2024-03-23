use super::canvas_drawing::*;
use super::drawing_state::*;
use super::texture::*;

use crate::filters::*;
use crate::pixel::*;
use crate::pixel_programs::*;
use crate::scanplan::*;
use crate::render::*;

use flo_canvas as canvas;

use std::sync::*;

///
/// A dynamic sprite is a sprite that is rendered to a texture on demand, where the texture's size is determined by the current render resolution
/// rather than being measured in pixels
///
#[derive(Clone)]
pub (crate) struct DynamicSprite {
    /// The namespace that the sprite is in
    namespace_id: canvas::NamespaceId,

    /// The ID of the sprite that this represents
    sprite_id: canvas::SpriteId,

    /// The region in sprite coordinates to render
    sprite_bounds: canvas::SpriteBounds,

    /// The size of the final texture, in canvas units
    canvas_bounds: canvas::CanvasSize,

    /// The filters that are applied to this dynamic sprite
    filters: Vec<Arc<dyn Send + Sync + Fn(U16LinearTexture) -> U16LinearTexture>>,

    /// The most recent render of this sprite (or None if it has never been rendered)
    last_render: Option<Arc<U16LinearTexture>>,

    /// The last pixel size used in canvas units, used to determine if this texture needs to be re-rendered
    last_pixel_size: f64,

    /// The layer edit count used the last time this sprite was re-rendered
    last_render_layer_count: usize,
}

impl DynamicSprite {
    ///
    /// True if the last render has changed and needs to be redone
    ///
    #[inline]
    fn has_changed<TPixel, const N: usize>(&self, drawing: &CanvasDrawing<TPixel, N>) -> bool
    where
        TPixel: Pixel<N>
    {
        if self.last_render.is_none() {
            true
        } else if self.last_pixel_size != drawing.height_pixels {
            true
        } else if let Some(sprite_layer_handle) = drawing.sprites.get(&(self.namespace_id, self.sprite_id)) {
            if let Some(sprite_layer) = drawing.layers.get(sprite_layer_handle.0) {
                self.last_render_layer_count != sprite_layer.edit_count
            } else {
                false
            }
        } else {
            false
        }
    }

    ///
    /// Immediately renders this dynamic sprite
    ///
    fn render<TPixel, const N: usize>(&self, drawing: &mut CanvasDrawing<TPixel, N>) -> U16LinearTexture 
    where
        TPixel: Pixel<N>
    {
        let sprites     = &drawing.sprites;
        let layers      = &mut drawing.layers;
        let origin      = self.sprite_bounds.0;
        let size        = self.sprite_bounds.1;

        // Figure out the width and height of the new texture
        let pixel_height    = (drawing.height_pixels * 0.5) as f32;
        let width           = self.canvas_bounds.0 * pixel_height;
        let height          = self.canvas_bounds.1 * pixel_height;
        let width           = width.abs().ceil() as usize;
        let height          = height.abs().ceil() as usize;

        // Fetch the sprite corresponding to the sprite ID
        let sprite_layer = sprites.get(&(self.namespace_id, self.sprite_id))
            .and_then(|layer_handle| layers.get_mut(layer_handle.0));

        let sprite_layer = if let Some(sprite_layer) = sprite_layer {
            sprite_layer
        } else {
            return U16LinearTexture::from_pixels(1, 1, vec![0u16; 4])
        };

        // The sprite transform maps from the sprite coordinates to the range -1,1 to 1,1
        let sprite_transform = sprite_layer.last_transform;

        // Upper and lower bounds are the coordinates that are the bounds of the area to render to the texture
        let lower_bounds    = sprite_transform.transform_point(origin.0 as _, origin.1 as _);
        let upper_bounds    = sprite_transform.transform_point((origin.0 + size.0) as _, (origin.1 + size.1) as _);
        let bounds_w        = upper_bounds.0-lower_bounds.0;
        let bounds_h        = upper_bounds.1-lower_bounds.1;

        // Map the bounds to the texture pixels
        let to_texture_pixels = canvas::Transform2D::scale((width as f32)/bounds_w, (height as f32)/bounds_h) * canvas::Transform2D::translate(-lower_bounds.0, -lower_bounds.1);

        // Transform the edges from the layer to prepare them to render
        // TODO: could be better to use a transform in the renderer instead (which is what the canvas renderer does)
        sprite_layer.edges.prepare_to_render();
        let mut edges = sprite_layer.edges.transform(&to_texture_pixels);
        edges.prepare_to_render();

        // Create a background scan planner using the default pixel colour for the sprite
        // We need a background planner to clear the background colour
        let background_col  = SolidColorData(TPixel::default());
        let background_data = drawing.program_cache.program_cache.store_program_data(&drawing.program_cache.solid_color, &mut drawing.program_data_cache, background_col);
        let background      = BackgroundScanPlanner::new(ShardScanPlanner::default(), background_data);

        // Render the new texture
        let pixels = {
            // Use the scan planner to create a frame renderer
            let mut pixels      = vec![0u16; width*height*4];
            let renderer        = EdgePlanRegionRenderer::new(background, ScanlineRenderer::new(drawing.program_runner(height as _)));
            let frame_renderer  = U16LinearFrameRenderer::new(renderer);

            // Call the frame renderer to generate the pixels
            let region = FrameSize { width, height };
            frame_renderer.render(&region, &edges, &mut pixels);

            pixels
        };

        // Release the data we were using in the planner
        drawing.program_data_cache.release_program_data(background_data);

        // Save the pixels to the texture
        let texture = U16LinearTexture::from_pixels(width, height, pixels);
        texture
    }

    ///
    /// Retrieves the texture (possibly rendering it if needed)
    ///
    pub fn get_u16_texture<TPixel, const N: usize>(&mut self, drawing: &mut CanvasDrawing<TPixel, N>) -> Arc<U16LinearTexture>
    where
        TPixel: Pixel<N>
    {
        if self.has_changed(drawing) {
            // Clear the last texture
            self.last_render = None;

            // Render a new texture
            let mut new_texture = self.render(drawing);

            // Apply filters
            for filter in self.filters.iter() {
                new_texture = filter(new_texture);
            }

            // Store the new texture
            self.last_render                = Some(Arc::new(new_texture));

            self.last_pixel_size            = drawing.height_pixels;
            self.last_render_layer_count    = if let Some(sprite_layer_handle) = drawing.sprites.get(&(self.namespace_id, self.sprite_id)) {
                if let Some(layer) = drawing.layers.get(sprite_layer_handle.0) {
                    layer.edit_count
                } else {
                    usize::MAX
                }
            } else {
                usize::MAX
            };
        }

        // Return the most recent render (which should always be up to date at this point)
        self.last_render.as_ref().unwrap().clone()
    }

    ///
    /// Adds a filter to the dynamic sprite
    ///
    pub fn apply_filter<TPixel, const N: usize>(&mut self, filter: impl 'static + Send + Sync + PixelFilter<Pixel=TPixel>)
    where
        TPixel: Pixel<N>
    {
        // Clear the 'last rendered' value (we could also apply the filter to it immediately if it exists to avoid a complete re-render)
        self.last_render = None;

        // Add the filter
        self.filters.push(Arc::new(move |input_texture| {
            let width  = input_texture.width();
            let height = input_texture.height();

            let mut converted_pixels = vec![0u16; width * height * 4];

            // Write to the output by filtering the input texture
            for (ypos, pixel_line) in filter_texture(&input_texture, &filter).enumerate() {
                let start_pos   = width * ypos * 4;
                let end_pos     = start_pos + width * 4;

                let target_pixel = U16LinearPixel::u16_slice_as_linear_pixels(&mut converted_pixels[start_pos..end_pos]);
                TPixel::to_linear_colorspace(&pixel_line, target_pixel);
            }

            // Create the texture from the result
            let linear_texture = U16LinearTexture::from_pixels(width, height, converted_pixels);
            linear_texture
        }));
    }
}

impl<TPixel, const N: usize> CanvasDrawing<TPixel, N>
where
    TPixel: 'static + Send + Sync + Pixel<N>,
{
    ///
    /// Creates a dynamic texture.
    ///
    /// These are rendered at the resolution of the output, and are re-rendered whenever the resolution or the sprite changes.
    /// Filters are re-applied when re-rendering. The textures are rendered just before they are used and are not re-rendered
    /// unless the parameters change.
    ///
    pub fn texture_create_dynamic_sprite(&mut self, texture_id: canvas::TextureId, sprite_id: canvas::SpriteId, bounds: canvas::SpriteBounds, size: canvas::CanvasSize) {
        let lower_left  = self.current_state.transform.transform_point(0.0, 0.0);
        let upper_right = self.current_state.transform.transform_point(size.0, size.1);

        let size        = canvas::CanvasSize((upper_right.0 - lower_left.0).abs(), (upper_right.1 - lower_left.1).abs());

        // Create a structure to represent the dynamic sprite, which is not currently rendered
        let new_sprite = DynamicSprite {
            namespace_id:               self.current_namespace,
            sprite_id:                  sprite_id,
            sprite_bounds:              bounds,
            canvas_bounds:              size,
            filters:                    vec![],
            last_render:                None,
            last_pixel_size:            0.0,
            last_render_layer_count:    0
        };

        // Store as a dynamic texture
        let texture = Texture {
            pixels: TexturePixels::DynamicSprite(Arc::new(Mutex::new(new_sprite))),
            fill_alpha: 1.0,
        };
        self.textures.insert((self.current_namespace, texture_id), texture);
    }
}

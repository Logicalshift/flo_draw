use super::canvas_renderer::*;

use crate::render_texture::*;
use crate::texture_render_request::*;

use flo_canvas as canvas;
use flo_render as render;

use std::sync::*;

impl CanvasRenderer {
    ///
    /// Dispatches a texture operation
    ///
    #[inline]
    pub (super) fn tes_texture(&mut self, texture_id: canvas::TextureId, op: canvas::TextureOp) {
        use canvas::TextureOp::*;
        use canvas::{TextureSize, TextureFormat};

        match op {
            Create(TextureSize(w, h), TextureFormat::Rgba)              => self.tes_texture_create_rgba(texture_id, w, h),
            Free                                                        => self.tes_texture_free(texture_id),
            SetBytes(position, size, bytes)                             => self.tes_texture_set_bytes(texture_id, position, size, bytes),
            SetFromSprite(sprite_id, bounds)                            => self.tes_texture_set_from_sprite(texture_id, sprite_id, bounds),
            CreateDynamicSprite(sprite_id, sprite_bounds, canvas_size)  => self.tes_texture_create_dynamic_sprite(texture_id, sprite_id, sprite_bounds, canvas_size),
            FillTransparency(alpha)                                     => self.tes_texture_fill_transparency(texture_id, alpha),
            Copy(target_texture_id)                                     => self.tes_texture_copy(texture_id, target_texture_id),
            Filter(filter)                                              => { todo!() },
        }
    }

    ///
    /// Creates or replaces a texture
    ///
    fn tes_texture_create_rgba(&mut self, texture_id: canvas::TextureId, width: u32, height: u32) {
        self.core.sync(|core| {
            // If the texture ID was previously in use, reduce the usage count
            let render_texture = if let Some(old_render_texture) = core.canvas_textures.get(&texture_id) {
                let old_render_texture  = old_render_texture.into();
                let usage_count         = core.used_textures.get_mut(&old_render_texture);

                if usage_count == Some(&mut 1) {
                    // Leave the usage count as is and reallocate the existing texture
                    // The 1 usage is the rendered version of this texture
                    old_render_texture
                } else {
                    // Reduce the usage count
                    usage_count.map(|usage_count| *usage_count -=1);

                    // Allocate a new texture
                    core.allocate_texture()
                }
            } else {
                // Unused texture ID: allocate a new texture
                core.allocate_texture()
            };

            // Add this as a texture with a usage count of 1
            core.canvas_textures.insert(texture_id, RenderTexture::Loading(render_texture));
            core.used_textures.insert(render_texture, 1);
            core.texture_size.insert(render_texture, render::Size2D(width as _, height as _));

            // Create the texture in the setup actions
            core.setup_actions.push(render::RenderAction::CreateTextureBgra(render_texture, render::Size2D(width as _, height as _)));
        });
    }

    ///
    /// Release an existing texture
    ///
    fn tes_texture_free(&mut self, texture_id: canvas::TextureId) {
        self.core.sync(|core| {
            // If the texture ID was previously in use, reduce the usage count
            if let Some(old_render_texture) = core.canvas_textures.get(&texture_id) {
                let old_render_texture = old_render_texture.into();
                core.used_textures.get_mut(&old_render_texture)
                    .map(|usage_count| *usage_count -=1);
            }

            // Unmap the texture
            core.canvas_textures.remove(&texture_id);
        });
    }

    ///
    /// Updates an existing texture
    ///
    fn tes_texture_set_bytes(&mut self, texture_id: canvas::TextureId, canvas::TexturePosition(x, y): canvas::TexturePosition, canvas::TextureSize(width, height): canvas::TextureSize, bytes: Arc<Vec<u8>>) {
        self.core.sync(|core| {
            // Create a canvas renderer job that will write these bytes to the texture
            if let Some(render_texture) = core.canvas_textures.get(&texture_id) {
                let mut render_texture = *render_texture;

                // If the texture has one used count and is in a 'ready' state, switch it back to 'loading' (nothing has rendered it)
                if let RenderTexture::Ready(render_texture_id) = &render_texture {
                    if core.used_textures.get(render_texture_id) == Some(&1) {
                        core.canvas_textures.insert(texture_id, RenderTexture::Loading(*render_texture_id));
                        render_texture = RenderTexture::Loading(*render_texture_id);
                    }
                }

                // The texture is updated in a setup action
                match render_texture {
                    RenderTexture::Ready(render_texture)    => {
                        // Generate a copy of the texture and write to that instead ('Ready' textures are already rendered elsewhere)
                        let copy_texture_id = core.allocate_texture();

                        // Stop using the initial texture, and create a new copy that's 'Loading'
                        core.used_textures.get_mut(&render_texture).map(|usage_count| *usage_count -= 1);
                        core.used_textures.insert(copy_texture_id, 1);
                        core.canvas_textures.insert(texture_id, RenderTexture::Loading(copy_texture_id));

                        // Generate a copy
                        core.texture_size.insert(copy_texture_id, core.texture_size.get(&render_texture).unwrap().clone());
                        core.setup_actions.push(render::RenderAction::CopyTexture(render_texture, copy_texture_id));

                        // Update the data in the copy
                        core.setup_actions.push(render::RenderAction::WriteTextureData(copy_texture_id, render::Position2D(x as _, y as _), render::Position2D((x+width) as _, (y+height) as _), bytes));
                    }

                    RenderTexture::Loading(render_texture)  => {
                        // Use the existing texture
                        core.setup_actions.push(render::RenderAction::WriteTextureData(render_texture, render::Position2D(x as _, y as _), render::Position2D((x+width) as _, (y+height) as _), bytes));
                    }
                }
            }
        });
    }

    ///
    /// Render a texture from a sprite
    ///
    fn tes_texture_set_from_sprite(&mut self, texture_id: canvas::TextureId, sprite_id: canvas::SpriteId, bounds: canvas::SpriteBounds) {
        let canvas::SpriteBounds(canvas::SpritePosition(x, y), canvas::SpriteSize(w, h)) = bounds;

        self.core.sync(|core| {
            // Specify this as a texture that needs to be loaded by rendering from a layer
            if let (Some(render_texture), Some(sprite_layer_handle)) = (core.canvas_textures.get(&texture_id), core.sprites.get(&sprite_id)) {
                let mut render_texture  = *render_texture;
                let sprite_layer_handle = *sprite_layer_handle;

                // If the texture has one used count and is in a 'ready' state, switch it back to 'loading' (nothing has rendered it)
                if let RenderTexture::Ready(render_texture_id) = &render_texture {
                    if core.used_textures.get(render_texture_id) == Some(&1) {
                        core.canvas_textures.insert(texture_id, RenderTexture::Loading(*render_texture_id));
                        render_texture = RenderTexture::Loading(*render_texture_id);
                    }
                }

                // This texture needs to be marked to be rendered after the setup is completed
                let texture_id = match render_texture {
                    RenderTexture::Ready(render_texture)    => {
                        // Create a blank texture, and move back to the loading state
                        let new_texture_id = core.allocate_texture();

                        // Stop using the initial texture, and create a new copy that's 'Loading'
                        core.used_textures.get_mut(&render_texture).map(|usage_count| *usage_count -= 1);
                        core.used_textures.insert(new_texture_id, 1);
                        core.canvas_textures.insert(texture_id, RenderTexture::Ready(new_texture_id));

                        // Generate a copy
                        core.texture_size.insert(new_texture_id, core.texture_size.get(&render_texture).unwrap().clone());
                        core.setup_actions.push(render::RenderAction::CopyTexture(render_texture, new_texture_id));

                        // Write to the new texture
                        new_texture_id
                    }

                    RenderTexture::Loading(render_texture)  => {
                        // Use the existing texture
                        core.canvas_textures.insert(texture_id, RenderTexture::Ready(render_texture));
                        render_texture
                    }
                };

                // Cause the stream to render the sprite to the texture at the start of the next frame
                core.layer_textures.insert(texture_id, TextureRenderRequest::FromSprite(texture_id, sprite_layer_handle, canvas::SpriteBounds(canvas::SpritePosition(x, y), canvas::SpriteSize(w, h))));
            }
        });
    }

    ///
    /// Render a texture from a sprite, updating it dynamically as the canvas resolution changes
    ///
    fn tes_texture_create_dynamic_sprite(&mut self, texture_id: canvas::TextureId, sprite_id: canvas::SpriteId, sprite_bounds: canvas::SpriteBounds, canvas_size: canvas::CanvasSize) {
        self.core.sync(|core| {
            core.layer(self.current_layer).update_transform(&self.active_transform);

            if let Some(sprite_layer_handle) = core.sprites.get(&sprite_id) {
                let sprite_layer_handle = *sprite_layer_handle;
                let transform           = self.active_transform;

                // If the texture ID was previously in use, reduce the usage count
                let render_texture_id = if let Some(old_render_texture) = core.canvas_textures.get(&texture_id) {
                    let old_render_texture  = old_render_texture.into();
                    let usage_count         = core.used_textures.get_mut(&old_render_texture);

                    if usage_count == Some(&mut 1) {
                        // Leave the usage count as is and reallocate the existing texture
                        // The 1 usage is the rendered version of this texture
                        old_render_texture
                    } else {
                        // Reduce the usage count
                        usage_count.map(|usage_count| *usage_count -=1);

                        // Allocate a new texture
                        core.allocate_texture()
                    }
                } else {
                    // Unused texture ID: allocate a new texture
                    core.allocate_texture()
                };

                // Add this as a texture with a usage count of 1
                core.canvas_textures.insert(texture_id, RenderTexture::Loading(render_texture_id));
                core.used_textures.insert(render_texture_id, 1);
                core.texture_size.insert(render_texture_id, render::Size2D(1 as _, 1 as _));
                core.dynamic_texture_state.remove(&render_texture_id);

                // Specify as a dynamic texture
                core.layer_textures.insert(render_texture_id, TextureRenderRequest::DynamicTexture(render_texture_id, sprite_layer_handle, sprite_bounds, canvas_size, transform));
            }
        });
    }

    ///
    /// Sets the transparency to use when drawing a particular texture
    ///
    fn tes_texture_fill_transparency(&mut self, texture_id: canvas::TextureId, alpha: f32) {
        self.core.sync(|core| {
            core.texture_alpha.insert(texture_id, alpha);
            let layer                   = core.layer(self.current_layer);

            if layer.state.fill_color.texture_id() == Some(texture_id) {
                layer.state.fill_color  = layer.state.fill_color.with_texture_alpha(alpha);
            }
        });
    }

    ///
    /// Generates a copy from one texture to another
    ///
    fn tes_texture_copy(&mut self, source_texture_id: canvas::TextureId, target_texture_id: canvas::TextureId) {
        self.core.sync(|core| {
            // Get the source texture we're copying from
            let source_render_texture   = if let Some(texture) = core.canvas_textures.get(&source_texture_id) { *texture } else { return; };
            let source_texture_size     = *core.texture_size.get(&source_render_texture.into()).unwrap();

            // If the target is an existing texture, need to reduce the usage count
            if let Some(old_render_texture) = core.canvas_textures.get(&target_texture_id) {
                let old_render_texture = old_render_texture.into();
                core.used_textures.get_mut(&old_render_texture)
                    .map(|usage_count| *usage_count -=1);
            }

            // Allocate a new texture as the target (it's loading for the moment as nothing is actually using it)
            let target_render_texture   = core.allocate_texture();
 
            core.canvas_textures.insert(target_texture_id, RenderTexture::Loading(target_render_texture));
            core.used_textures.insert(target_render_texture, 1);
            core.texture_size.insert(target_render_texture, source_texture_size);

            // Generate the copy instruction
            core.setup_actions.push(render::RenderAction::CopyTexture(source_render_texture.into(), target_render_texture));
            core.setup_actions.push(render::RenderAction::CreateMipMaps(target_render_texture));
        });
    }
}
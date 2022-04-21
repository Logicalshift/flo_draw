use crate::fill_state::*;
use crate::render_entity::*;
use crate::renderer_worker::*;

use super::canvas_renderer::*;
use super::tessellate_build_path::*;

use flo_stream::*;
use flo_canvas as canvas;
use flo_render as render;

use std::mem;

const BATCH_SIZE: usize = 20;

impl CanvasRenderer {
    ///
    /// Fill the current path
    ///
    pub (super) async fn tes_fill(&mut self, path_state: &mut PathState, job_publisher: &mut SinglePublisher<Vec<CanvasJob>>, pending_jobs: &mut Vec<CanvasJob>) {
        // Update the active path if the builder exists
        path_state.build();

        // Publish the fill job to the tessellators
        if let Some(path) = &path_state.current_path {
            let path                = path.clone();
            let layer_id            = self.current_layer;
            let entity_id           = self.next_entity_id;
            let viewport_height     = self.viewport_size.1;
            let active_transform    = &self.active_transform;

            self.next_entity_id += 1;

            let job = self.core.sync(move |core| {
                let layer = core.layer(layer_id);

                // Update the transformation matrix
                layer.update_transform(active_transform);

                // Rendering in a blend mode other than source over sets the 'commit before rendering' flag for this layer
                if layer.state.blend_mode != canvas::BlendMode::SourceOver {
                    layer.commit_before_rendering = true;
                }

                // If the shader state has changed, generate the operations needed to use that shader state
                if path_state.fill_state != layer.state.fill_color {
                    // Update the active fill state to match that of the layer
                    match layer.state.fill_color {
                        FillState::None | FillState::Color(_) => { 
                            layer.render_order.push(RenderEntity::SetFlatColor);
                        }

                        FillState::Texture(render_texture, matrix, repeat, alpha) => {
                            // Increase the usage count for this texture
                            core.used_textures.get_mut(&render_texture)
                                .map(|usage_count| *usage_count += 1);

                            // Add to the layer
                            core.layer(layer_id).render_order.push(RenderEntity::SetFillTexture(render_texture, matrix, repeat, alpha));
                        }

                        FillState::LinearGradient(gradient_texture, matrix, repeat, alpha) => {
                            // Increase the usage count for the texture
                            core.used_textures.get_mut(&gradient_texture)
                                .map(|usage_count| *usage_count += 1);

                            // Add to the layer
                            core.layer(layer_id).render_order.push(RenderEntity::SetFillGradient(gradient_texture, matrix, repeat, alpha));
                        }
                    }

                    path_state.dash_pattern = vec![];
                    path_state.fill_state   = core.layer(layer_id).state.fill_color.clone();
                } else if !path_state.dash_pattern.is_empty() {
                    // Ensure there's no dash pattern
                    layer.render_order.push(RenderEntity::SetFlatColor);
                    path_state.dash_pattern = vec![];
                    path_state.fill_state   = layer.state.fill_color.clone();
                }

                // Create the render entity in the tessellating state
                let layer               = core.layer(layer_id);
                let scale_factor        = layer.state.tolerance_scale_factor(viewport_height);
                let color               = layer.state.fill_color.clone();
                let fill_rule           = layer.state.winding_rule;
                let entity_index        = layer.render_order.len();
                let transform           = layer.state.current_matrix;

                layer.render_order.push(RenderEntity::Tessellating(entity_id));
                layer.state.modification_count += 1;

                let entity          = LayerEntityRef { layer_id, entity_index, entity_id };

                // Create the canvas job
                CanvasJob::Fill { path, fill_rule, color, scale_factor, transform, entity }
            });

            pending_jobs.push(job);
            if pending_jobs.len() >= BATCH_SIZE {
                let mut jobs_to_send = vec![];
                mem::swap(&mut jobs_to_send, pending_jobs);

                job_publisher.publish(jobs_to_send).await;
            }
        }
    }

    ///
    /// Draw a line around the current path
    ///
    pub (super) async fn tes_stroke(&mut self, path_state: &mut PathState, job_publisher: &mut SinglePublisher<Vec<CanvasJob>>, pending_jobs: &mut Vec<CanvasJob>) {
        // Update the active path if the builder exists
        path_state.build();

        // Publish the job to the tessellators
        if let Some(path) = &path_state.current_path {
            let path                = path.clone();
            let layer_id            = self.current_layer;
            let entity_id           = self.next_entity_id;
            let viewport_height     = self.viewport_size.1;
            let active_transform    = &self.active_transform;
            let dash_pattern        = &mut path_state.dash_pattern;
            let fill_state          = &mut path_state.fill_state;

            self.next_entity_id += 1;

            let job = self.core.sync(move |core| {
                let layer = core.layer(layer_id);

                // Rendering in a blend mode other than source over sets the 'commit before rendering' flag for this layer
                if layer.state.blend_mode != canvas::BlendMode::SourceOver {
                    layer.commit_before_rendering = true;
                }

                // Update the transformation matrix
                layer.update_transform(active_transform);

                // Reset the fill state to 'flat colour' if needed
                match fill_state {
                    FillState::None     | 
                    FillState::Color(_) => { }
                    _                   => { layer.render_order.push(RenderEntity::SetFlatColor) }
                }

                *fill_state = FillState::None;

                // Apply the dash pattern, if it's different
                if *dash_pattern != layer.state.stroke_settings.dash_pattern {
                    layer.render_order.push(RenderEntity::SetDashPattern(layer.state.stroke_settings.dash_pattern.clone()));
                    *dash_pattern = layer.state.stroke_settings.dash_pattern.clone();
                }

                // Create the render entity in the tessellating state
                let scale_factor        = layer.state.tolerance_scale_factor(viewport_height);
                let mut stroke_options  = layer.state.stroke_settings.clone();
                let entity_index        = layer.render_order.len();
                let transform           = layer.state.current_matrix;

                // When drawing to the erase layer (DesintationOut blend mode), all colour components are alpha components
                let color                   = stroke_options.stroke_color;
                stroke_options.stroke_color = if layer.state.blend_mode == canvas::BlendMode::DestinationOut { render::Rgba8([color.0[3], color.0[3], color.0[3], color.0[3]]) } else { color };

                layer.render_order.push(RenderEntity::Tessellating(entity_id));
                layer.state.modification_count += 1;

                let entity          = LayerEntityRef { layer_id, entity_index, entity_id };

                // Create the canvas job
                CanvasJob::Stroke { path, stroke_options, scale_factor, transform, entity }
            });

            pending_jobs.push(job);
            if pending_jobs.len() >= BATCH_SIZE {
                let mut jobs_to_send = vec![];
                mem::swap(&mut jobs_to_send, pending_jobs);

                job_publisher.publish(jobs_to_send).await;
            }
        }
    }

    ///
    /// Clip to the currently set path
    ///
    pub (super) async fn tes_clip(&mut self, path_state: &mut PathState, job_publisher: &mut SinglePublisher<Vec<CanvasJob>>, pending_jobs: &mut Vec<CanvasJob>) {
        // Update the active path if the builder exists
        path_state.build();

        // Publish the fill job to the tessellators
        if let Some(path) = &path_state.current_path {
            let path                = path.clone();
            let layer_id            = self.current_layer;
            let entity_id           = self.next_entity_id;
            let viewport_height     = self.viewport_size.1;
            let active_transform    = &self.active_transform;

            self.next_entity_id += 1;

            let job = self.core.sync(move |core| {
                let layer = core.layer(layer_id);

                // Update the transformation matrix
                layer.update_transform(active_transform);

                // Create the render entity in the tessellating state
                let scale_factor        = layer.state.tolerance_scale_factor(viewport_height);
                let color               = render::Rgba8([255, 255, 255, 255]);
                let fill_rule           = layer.state.winding_rule;
                let entity_index        = layer.render_order.len();
                let transform           = *active_transform;

                // Update the clipping path and enable clipping
                layer.render_order.push(RenderEntity::Tessellating(entity_id));

                let entity          = LayerEntityRef { layer_id, entity_index, entity_id };

                // Create the canvas job
                CanvasJob::Clip { path, fill_rule, color, scale_factor, transform, entity }
            });

            pending_jobs.push(job);
            if pending_jobs.len() >= BATCH_SIZE {
                let mut jobs_to_send = vec![];
                mem::swap(&mut jobs_to_send, pending_jobs);

                job_publisher.publish(jobs_to_send).await;
            }
        }
    }

    ///
    /// Unset the clipping path
    ///
    pub fn tes_unclip(&mut self) {
        self.core.sync(|core| {
            let layer           = core.layer(self.current_layer);

            // Render the sprite
            layer.render_order.push(RenderEntity::DisableClipping);
        })
    }
}

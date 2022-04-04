use crate::fill_state::*;
use crate::render_entity::*;
use crate::renderer_worker::*;

use super::canvas_renderer::*;
use super::tessellate_build_path::*;

use flo_stream::*;
use flo_canvas as canvas;

use std::mem;

const BATCH_SIZE: usize = 20;

impl CanvasRenderer {
    /// Fill the current path
    pub (super) async fn tes_fill(&mut self, path_state: &mut PathState, fill_state: &mut FillState, dash_pattern: &mut Vec<f32>, job_publisher: &mut SinglePublisher<Vec<CanvasJob>>, pending_jobs: &mut Vec<CanvasJob>) {
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
                if *fill_state != layer.state.fill_color {
                    // Update the active fill state to match that of the layer
                    match layer.state.fill_color {
                        FillState::None | FillState::Color(_) => { 
                            layer.render_order.push(RenderEntity::SetFlatColor);
                        }

                        FillState::Texture(texture_id, matrix, repeat, alpha) => {
                            // Finish/get the render texture
                            if let Some(render_texture) = core.texture_for_rendering(texture_id) {
                                // Increase the usage count for this texture
                                core.used_textures.get_mut(&render_texture)
                                    .map(|usage_count| *usage_count += 1);

                                // Add to the layer
                                core.layer(layer_id).render_order.push(RenderEntity::SetFillTexture(render_texture, matrix, repeat, alpha));
                            } else {
                                // Texture is not set up
                                core.layer(layer_id).render_order.push(RenderEntity::SetFlatColor);
                            }
                        }

                        FillState::LinearGradient(gradient_id, matrix, repeat, alpha) => {
                            // Finish/get the texture for the gradient
                            if let Some(gradient_texture) = core.gradient_for_rendering(gradient_id) {
                                // Increase the usage count for the texture
                                core.used_textures.get_mut(&gradient_texture)
                                    .map(|usage_count| *usage_count += 1);

                                // Add to the layer
                                core.layer(layer_id).render_order.push(RenderEntity::SetFillGradient(gradient_texture, matrix, repeat, alpha));
                            } else {
                                // Gradient is not set up
                                core.layer(layer_id).render_order.push(RenderEntity::SetFlatColor);
                            }
                        }
                    }


                    *dash_pattern   = vec![];
                    *fill_state     = core.layer(layer_id).state.fill_color.clone();
                } else if *dash_pattern != vec![] {
                    // Ensure there's no dash pattern
                    layer.render_order.push(RenderEntity::SetFlatColor);
                    *dash_pattern   = vec![];
                    *fill_state     = layer.state.fill_color.clone();
                }

                // Create the render entity in the tessellating state
                let layer               = core.layer(layer_id);
                let scale_factor        = layer.state.tolerance_scale_factor(viewport_height);
                let color               = layer.state.fill_color.clone();
                let fill_rule           = layer.state.winding_rule;
                let entity_index        = layer.render_order.len();
                let transform           = *active_transform;

                // When drawing to the erase layer (DesintationOut blend mode), all colour components are alpha components
                let color               = if layer.state.blend_mode == canvas::BlendMode::DestinationOut { color.all_channel_alpha() } else { color };

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
}

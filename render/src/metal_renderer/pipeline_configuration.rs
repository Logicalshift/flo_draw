use crate::action::*;

use metal;

///
/// Represents the configuration of a render pipeline for Metal
///
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PipelineConfiguration {
    ///
    /// The blend mode to use for this configuration
    ///
    pub blend_mode: BlendMode,

    ///
    /// The name of the vertex shader to use
    ///
    pub vertex_shader: String,

    ///
    /// The name of the fragment shader to use
    ///
    pub fragment_shader: String
}

impl Default for PipelineConfiguration {
    fn default() -> PipelineConfiguration {
        PipelineConfiguration {
            blend_mode:         BlendMode::SourceOver,
            vertex_shader:      String::from("simple_vertex"),
            fragment_shader:    String::from("simple_fragment")
        }
    }
}

impl PipelineConfiguration {
    ///
    /// Creates a pipeline state from a configuration
    ///
    pub fn to_pipeline_state(&self, device: &metal::Device, library: &metal::Library) -> metal::RenderPipelineState {
        let descriptor      = metal::RenderPipelineDescriptor::new();

        // Load the shader
        let vertex_shader   = library.get_function(&self.vertex_shader, None).unwrap();
        let fragment_shader = library.get_function(&self.fragment_shader, None).unwrap();

        descriptor.set_vertex_function(Some(&vertex_shader));
        descriptor.set_fragment_function(Some(&fragment_shader));

        // Set the blend mode
        use self::BlendMode::*;
        use metal::MTLBlendFactor::{SourceAlpha, OneMinusSourceAlpha, One, DestinationAlpha, OneMinusDestinationAlpha, Zero, OneMinusSourceColor, OneMinusDestinationColor};
        let (src_rgb, src_alpha, dst_rgb, dst_alpha) = match self.blend_mode {
            SourceOver                      => (SourceAlpha, OneMinusSourceAlpha, One, OneMinusSourceAlpha),
            DestinationOver                 => (OneMinusDestinationAlpha, DestinationAlpha, OneMinusDestinationAlpha, One),
            SourceIn                        => (DestinationAlpha, Zero, DestinationAlpha, Zero),
            DestinationIn                   => (Zero, SourceAlpha, Zero, SourceAlpha),
            SourceOut                       => (Zero, OneMinusDestinationAlpha, Zero, OneMinusDestinationAlpha),
            DestinationOut                  => (Zero, OneMinusSourceAlpha, Zero, OneMinusSourceAlpha),
            SourceATop                      => (OneMinusDestinationAlpha, SourceAlpha, OneMinusDestinationAlpha, SourceAlpha),
            DestinationATop                 => (OneMinusDestinationAlpha, OneMinusSourceAlpha, OneMinusDestinationAlpha, OneMinusSourceAlpha),

            AllChannelAlphaSourceOver       => (One, OneMinusSourceColor, One, OneMinusSourceAlpha),
            AllChannelAlphaDestinationOver  => (OneMinusDestinationColor, One, OneMinusDestinationAlpha, One)
        };

        descriptor.color_attachments().object_at(0).unwrap().set_blending_enabled(true);
        descriptor.color_attachments().object_at(0).unwrap().set_source_rgb_blend_factor(src_rgb);
        descriptor.color_attachments().object_at(0).unwrap().set_destination_rgb_blend_factor(dst_rgb);
        descriptor.color_attachments().object_at(0).unwrap().set_source_alpha_blend_factor(src_alpha);
        descriptor.color_attachments().object_at(0).unwrap().set_destination_alpha_blend_factor(dst_alpha);

        // Create the state
        device.new_render_pipeline_state(&descriptor).unwrap()
    }
}

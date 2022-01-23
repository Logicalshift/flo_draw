use crate::action::*;

use metal;

///
/// Represents the configuration of a render pipeline for Metal
///
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PipelineConfiguration {
    ///
    /// The sample count for this pipeline configuration
    ///
    pub sample_count: u64,

    ///
    /// The pixel format for this pipeline configuration
    ///
    pub pixel_format: metal::MTLPixelFormat,

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
            sample_count:       1,
            pixel_format:       metal::MTLPixelFormat::BGRA8Unorm,
            blend_mode:         BlendMode::SourceOver,
            vertex_shader:      String::from("simple_vertex"),
            fragment_shader:    String::from("simple_fragment")
        }
    }
}

impl PipelineConfiguration {
    ///
    /// Creates a default pipeline configuration for rendering to the specified texture
    ///
    pub fn for_texture(texture: &metal::Texture) -> PipelineConfiguration {
        let mut pipeline_config = Self::default();
        pipeline_config.update_for_texture(texture);

        pipeline_config
    }

    ///
    /// Reads the properties of a texture and sets up this configuration to be appropriate for rendering to it
    ///
    pub fn update_for_texture(&mut self, texture: &metal::Texture) {
        self.sample_count = texture.sample_count();
        self.pixel_format = texture.pixel_format();
    }

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
        descriptor.set_sample_count(self.sample_count);

        // Set the blend mode
        use self::BlendMode::*;
        use metal::MTLBlendFactor::{SourceAlpha, OneMinusSourceAlpha, One, DestinationAlpha, DestinationColor, OneMinusDestinationAlpha, Zero, OneMinusSourceColor, OneMinusDestinationColor};
        let (src_rgb, dst_rgb, src_alpha, dst_alpha) = match self.blend_mode {
            SourceOver                      => (SourceAlpha, OneMinusSourceAlpha, One, OneMinusSourceAlpha),
            DestinationOver                 => (OneMinusDestinationAlpha, DestinationAlpha, OneMinusDestinationAlpha, One),
            SourceIn                        => (DestinationAlpha, Zero, DestinationAlpha, Zero),
            DestinationIn                   => (Zero, SourceAlpha, Zero, SourceAlpha),
            SourceOut                       => (Zero, OneMinusDestinationAlpha, Zero, OneMinusDestinationAlpha),
            DestinationOut                  => (Zero, OneMinusSourceAlpha, Zero, OneMinusSourceAlpha),
            SourceATop                      => (OneMinusDestinationAlpha, SourceAlpha, OneMinusDestinationAlpha, SourceAlpha),
            DestinationATop                 => (OneMinusDestinationAlpha, OneMinusSourceAlpha, OneMinusDestinationAlpha, OneMinusSourceAlpha),

            // Multiply is a*b. Here we multiply the source colour by the destination colour, then blend the destination back in again to take account of
            // alpha in the source layer (this version of multiply has no effect on the target alpha value: a more strict version might multiply those too)
            //
            // The source side is precalculated so that an alpha of 0 produces a colour of 1,1,1 to take account of transparency in the source.
            Multiply                        => (DestinationColor, Zero, Zero, One),

            // TODO: screen is 1-(1-a)*(1-b) which I think is harder to fake. If we precalculate (1-a) as the src in the shader
            Screen                          => (OneMinusDestinationColor, One, Zero, One),

            AllChannelAlphaSourceOver       => (One, OneMinusSourceColor, One, OneMinusSourceAlpha),
            AllChannelAlphaDestinationOver  => (OneMinusDestinationColor, One, OneMinusDestinationAlpha, One)
        };

        descriptor.color_attachments().object_at(0).unwrap().set_pixel_format(self.pixel_format);
        descriptor.color_attachments().object_at(0).unwrap().set_blending_enabled(true);
        descriptor.color_attachments().object_at(0).unwrap().set_source_rgb_blend_factor(src_rgb);
        descriptor.color_attachments().object_at(0).unwrap().set_destination_rgb_blend_factor(dst_rgb);
        descriptor.color_attachments().object_at(0).unwrap().set_source_alpha_blend_factor(src_alpha);
        descriptor.color_attachments().object_at(0).unwrap().set_destination_alpha_blend_factor(dst_alpha);

        // Create the state
        device.new_render_pipeline_state(&descriptor).unwrap()
    }
}

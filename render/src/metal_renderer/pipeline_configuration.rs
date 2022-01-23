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
    /// True if the source alpha has been pre-multiplied into its components
    ///
    pub source_is_premultiplied: bool,

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
            sample_count:               1,
            pixel_format:               metal::MTLPixelFormat::BGRA8Unorm,
            blend_mode:                 BlendMode::SourceOver,
            source_is_premultiplied:    false,
            vertex_shader:              String::from("simple_vertex"),
            fragment_shader:            String::from("simple_fragment")
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

        let fragment_shader = match self.blend_mode {
            BlendMode::Multiply => format!("{}_invert_color_alpha", self.fragment_shader),
            _                   => format!("{}", self.fragment_shader)
        };

        // Load the shader
        let vertex_shader   = library.get_function(&self.vertex_shader, None).unwrap();
        let fragment_shader = library.get_function(&fragment_shader, None).unwrap();

        descriptor.set_vertex_function(Some(&vertex_shader));
        descriptor.set_fragment_function(Some(&fragment_shader));
        descriptor.set_sample_count(self.sample_count);

        // Set the blend mode
        use self::BlendMode::*;
        use metal::MTLBlendFactor::{SourceAlpha, OneMinusSourceAlpha, One, DestinationAlpha, DestinationColor, OneMinusDestinationAlpha, Zero, OneMinusSourceColor, OneMinusDestinationColor};
        let (src_rgb, dst_rgb, src_alpha, dst_alpha) = match (self.blend_mode, self.source_is_premultiplied) {
            (SourceOver, false)                         => (SourceAlpha, OneMinusSourceAlpha, One, OneMinusSourceAlpha),
            (DestinationOver, false)                    => (OneMinusDestinationAlpha, DestinationAlpha, OneMinusDestinationAlpha, One),
            (SourceIn, false)                           => (DestinationAlpha, Zero, DestinationAlpha, Zero),
            (DestinationIn, false)                      => (Zero, SourceAlpha, Zero, SourceAlpha),
            (SourceOut, false)                          => (Zero, OneMinusDestinationAlpha, Zero, OneMinusDestinationAlpha),
            (DestinationOut, false)                     => (Zero, OneMinusSourceAlpha, Zero, OneMinusSourceAlpha),
            (SourceATop, false)                         => (OneMinusDestinationAlpha, SourceAlpha, OneMinusDestinationAlpha, SourceAlpha),
            (DestinationATop, false)                    => (OneMinusDestinationAlpha, OneMinusSourceAlpha, OneMinusDestinationAlpha, OneMinusSourceAlpha),

            // Multiply is a*b. Here we multiply the source colour by the destination colour, then blend the destination back in again to take account of
            // alpha in the source layer (this version of multiply has no effect on the target alpha value: a more strict version might multiply those too)
            //
            // The source side is precalculated so that an alpha of 0 produces a colour of 1,1,1 to take account of transparency in the source.
            (Multiply, false)                           => (DestinationColor, Zero, Zero, One),

            // TODO: screen is 1-(1-a)*(1-b) which I think is harder to fake. If we precalculate (1-a) as the src in the shader
            (Screen, false)                             => (OneMinusDestinationColor, One, Zero, One),

            (AllChannelAlphaSourceOver, false)          => (One, OneMinusSourceColor, One, OneMinusSourceAlpha),
            (AllChannelAlphaDestinationOver, false)     => (OneMinusDestinationColor, One, OneMinusDestinationAlpha, One),

            (SourceOver, true)                          => (One, OneMinusSourceAlpha, One, OneMinusSourceAlpha),
            (DestinationOver, true)                     => (OneMinusDestinationAlpha, DestinationAlpha, OneMinusDestinationAlpha, One),
            (SourceIn, true)                            => (DestinationAlpha, Zero, DestinationAlpha, Zero),
            (DestinationIn, true)                       => (Zero, SourceAlpha, Zero, SourceAlpha),
            (SourceOut, true)                           => (Zero, OneMinusDestinationAlpha, Zero, OneMinusDestinationAlpha),
            (DestinationOut, true)                      => (Zero, OneMinusSourceAlpha, Zero, OneMinusSourceAlpha),
            (SourceATop, true)                          => (OneMinusDestinationAlpha, SourceAlpha, OneMinusDestinationAlpha, SourceAlpha),
            (DestinationATop, true)                     => (OneMinusDestinationAlpha, OneMinusSourceAlpha, OneMinusDestinationAlpha, OneMinusSourceAlpha),
            (Multiply, true)                            => (DestinationColor, Zero, Zero, One),
            (Screen, true)                              => (OneMinusDestinationColor, One, Zero, One),

            (AllChannelAlphaSourceOver, true)           => (One, OneMinusSourceColor, One, OneMinusSourceAlpha),
            (AllChannelAlphaDestinationOver, true)      => (OneMinusDestinationColor, One, OneMinusDestinationAlpha, One),
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

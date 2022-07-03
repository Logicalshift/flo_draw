use super::wgpu_shader::*;
use super::shader_cache::*;
use crate::action::*;

use wgpu;

///
/// Description of a WGPU pipeline configuration (used to create the configuration and as a hash key)
///
#[derive(Clone, PartialEq, Eq, Hash)]
pub (crate) struct PipelineConfiguration {
    /// Format of the texture that this will render against
    pub (crate) texture_format: wgpu::TextureFormat,

    /// The identifier of the shader module to use (this defines both the vertex and the fragment shader, as well as the pipeline layout to use)
    pub (crate) shader_module: WgpuShader,

    /// The blending mode for this pipeline configuration
    pub (crate) blending_mode: BlendMode,
}

impl Default for PipelineConfiguration {
    fn default() -> PipelineConfiguration {
        PipelineConfiguration {
            texture_format: wgpu::TextureFormat::Bgra8Unorm,
            shader_module:  WgpuShader::default(),
            blending_mode:  BlendMode::SourceOver,
        }
    }
}

#[inline]
fn create_add_blend_state(rgb_src_factor: wgpu::BlendFactor, rgb_dst_factor: wgpu::BlendFactor, alpha_src_factor: wgpu::BlendFactor, alpha_dst_factor: wgpu::BlendFactor) -> wgpu::BlendState {
    wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: rgb_src_factor,
            dst_factor: rgb_dst_factor,
            operation:  wgpu::BlendOperation::Add,
        },

        alpha: wgpu::BlendComponent {
            src_factor: alpha_src_factor,
            dst_factor: alpha_dst_factor,
            operation:  wgpu::BlendOperation::Add,
        }
    }
}

#[inline]
fn create_op_blend_state(rgb_src_factor: wgpu::BlendFactor, rgb_dst_factor: wgpu::BlendFactor, alpha_src_factor: wgpu::BlendFactor, alpha_dst_factor: wgpu::BlendFactor, color_op: wgpu::BlendOperation, alpha_op: wgpu::BlendOperation) -> wgpu::BlendState {
    wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: rgb_src_factor,
            dst_factor: rgb_dst_factor,
            operation:  color_op,
        },

        alpha: wgpu::BlendComponent {
            src_factor: alpha_src_factor,
            dst_factor: alpha_dst_factor,
            operation:  alpha_op,
        }
    }
}

impl PipelineConfiguration {
    ///
    /// Retrieves the configured blend state for this pipeline
    ///
    #[inline]
    pub fn blend_state(&self) -> Option<wgpu::BlendState> {
        use self::BlendMode::*;
        use wgpu::BlendFactor::*;
        use wgpu::BlendOperation::*;

        match self.blending_mode {
            SourceOver          => Some(create_add_blend_state(SrcAlpha, OneMinusSrcAlpha, One, OneMinusSrcAlpha)),
            DestinationOver     => Some(create_add_blend_state(OneMinusDstAlpha, DstAlpha, OneMinusDstAlpha, One)),
            SourceIn            => Some(create_add_blend_state(DstAlpha, Zero, DstAlpha, Zero)),
            DestinationIn       => Some(create_add_blend_state(Zero, SrcAlpha, Zero, SrcAlpha)),
            SourceOut           => Some(create_add_blend_state(Zero, OneMinusDstAlpha, Zero, OneMinusDstAlpha)),
            DestinationOut      => Some(create_add_blend_state(Zero, OneMinusSrcAlpha, Zero, OneMinusSrcAlpha)),
            SourceATop          => Some(create_add_blend_state(OneMinusDstAlpha, SrcAlpha, OneMinusDstAlpha, SrcAlpha)),
            DestinationATop     => Some(create_add_blend_state(OneMinusDstAlpha, OneMinusSrcAlpha, OneMinusDstAlpha, OneMinusSrcAlpha)),

            // Multiply is a*b. Here we multiply the source colour by the destination colour, then blend the destination back in again to take account of
            // alpha in the source layer (this version of multiply has no effect on the target alpha value: a more strict version might multiply those too)
            //
            // The source side is precalculated so that an alpha of 0 produces a colour of 1,1,1 to take account of transparency in the source.
            Multiply            => Some(create_add_blend_state(Dst, Zero, Zero, One)),

            // TODO: screen is 1-(1-a)*(1-b) which I think is harder to fake. If we precalculate (1-a) as the src in the shader
            // then can multiply by ONE_MINUS_DST_COLOR to get (1-a)*(1-b). Can use One as our target colour, and then a 
            // reverse subtraction to get 1-(1-a)*(1-b)
            // (This implementation doesn't work: the One is 1*DST_COLOR and not 1 so this is currently 1*b-(1-a)*(1-b)
            // with shader support)
            Screen              => Some(create_op_blend_state(OneMinusDst, One, Zero, One, ReverseSubtract, Add)),

            AllChannelAlphaSourceOver       => Some(create_add_blend_state(One, OneMinusDst, One, OneMinusSrcAlpha)),
            AllChannelAlphaDestinationOver  => Some(create_add_blend_state(OneMinusDst, One, OneMinusDstAlpha, One)),
        }
    }

    ///
    /// Creates the colour target states for this pipeline
    ///
    #[inline]
    pub fn color_targets(&self) -> Vec<wgpu::ColorTargetState> {
        let blend_state = self.blend_state();

        vec![
            wgpu::ColorTargetState {
                format:     self.texture_format,
                blend:      blend_state,
                write_mask: wgpu::ColorWrites::ALL, 
            }
        ]
    }

    ///
    /// Creates the vertex state for this pipeline
    ///
    #[inline]
    pub fn vertex_state(&self, shader_cache: &mut ShaderCache<WgpuShader>) -> wgpu::VertexState<'_> {
        // TODO: needs the buffer layout and the shader module
        todo!()
    }

    ///
    /// Creates the fragment state for this render pipeline
    ///
    #[inline]
    pub fn fragment_state(&self, shader_cache: &mut ShaderCache<WgpuShader>) -> Option<wgpu::FragmentState<'_>> {
        let color_targets = self.color_targets();

        // TODO: needs shader module
        todo!()
    }

    ///
    /// Creates the pipeline layout for this render pipeline
    ///
    #[inline]
    pub fn pipeline_layout(&self) -> wgpu::PipelineLayoutDescriptor {
        wgpu::PipelineLayoutDescriptor {
            label:                  Some("pipeline_layout"),
            bind_group_layouts:     &[],
            push_constant_ranges:   &[],
        }
    }

    ///
    /// Creates the render pipeline descriptor for this render pipeline
    ///
    #[inline]
    pub fn render_pipeline_descriptor<'a>(&'a self, shader_cache: &mut ShaderCache<WgpuShader>, pipeline_layout: &'a wgpu::PipelineLayout) -> wgpu::RenderPipelineDescriptor<'a> {
        wgpu::RenderPipelineDescriptor {
            label:          Some("render_pipeline_descriptor"),
            layout:         Some(pipeline_layout),
            vertex:         self.vertex_state(shader_cache),
            fragment:       self.fragment_state(shader_cache),
            primitive:      wgpu::PrimitiveState::default(),
            depth_stencil:  None,
            multisample:    wgpu::MultisampleState::default(),
            multiview:      None,
        }
    }
}

use super::shader::*;
use super::shader_program::*;
use super::shader_uniforms::*;

///
/// The variants that every shader must have
///
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StandardShaderVariant {
    /// No clipping texture is applied to the shader
    NoClipping,

    /// A clipping texture is used to mask the rendering
    ClippingMask
}

///
/// The post-processing step to apply to the colour output of a shader
///
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorPostProcessingStep {
    /// The shader should not change the colour outputs
    NoPostProcessing,

    /// The shader should multiply its outputs by the alpha value
    MultiplyAlpha,

    /// The colour is blended so that at alpha (0), the RGB values are (1,1,1) - the inverse of pre-multiplications
    InvertColorAlpha,
}

///
/// The shader programs that are loaded by default into an OpenGL renderer
///
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StandardShaderProgram {
    /// Flat colour shader
    Simple(StandardShaderVariant, ColorPostProcessingStep),

    /// Renders fragments from a texture input
    Texture(StandardShaderVariant, ColorPostProcessingStep),

    /// Uses a 1D texture input to render a linear gradient fill
    LinearGradient(StandardShaderVariant, ColorPostProcessingStep),

    /// Uses a 1D texture to draw dashed lines
    DashedLine(StandardShaderVariant, ColorPostProcessingStep),

    /// Texture renderer that resolves MSAA textures 1-to-1 with the given number of samples
    MsaaResolve(u8, ColorPostProcessingStep)
}

impl StandardShaderVariant {
    ///
    /// Returns the #defines to declare in the shader program for this variant
    ///
    pub fn defines(&self) -> Vec<&str> {
        match self {
            StandardShaderVariant::NoClipping   => vec![],
            StandardShaderVariant::ClippingMask => vec!["CLIP_MASK"],
        }
    }
}

impl ColorPostProcessingStep {
    ///
    /// Returns the #defines to declare in the shader program for this variant
    ///
    pub fn defines(&self) -> Vec<&str> {
        match self {
            ColorPostProcessingStep::NoPostProcessing   => vec![],
            ColorPostProcessingStep::MultiplyAlpha      => vec!["MULTIPLY_ALPHA"],
            ColorPostProcessingStep::InvertColorAlpha   => vec!["INVERT_COLOUR_ALPHA"],
        }
    }
}

impl Default for StandardShaderProgram {
    fn default() -> Self {
        StandardShaderProgram::Simple(StandardShaderVariant::NoClipping, ColorPostProcessingStep::NoPostProcessing)
    }
}

impl StandardShaderProgram {
    ///
    /// Loads a shader with the specified vertex program, fragment program and set of defines
    ///
    fn load_shader(vertex_program: &str, vertex_attributes: &Vec<&str>, fragment_program: &str, fragment_attributes: &Vec<&str>, defines: &Vec<&str>) -> ShaderProgram<ShaderUniform> {
        let vertex_shader   = Shader::compile_with_defines(vertex_program, vertex_attributes, GlShaderType::Vertex, defines);
        let fragment_shader = Shader::compile_with_defines(fragment_program, fragment_attributes, GlShaderType::Fragment, defines);

        ShaderProgram::from_shaders(vec![vertex_shader, fragment_shader])
    }

    ///
    /// Creates the shader loader function (for use in the ShaderCollection) for the standard shader programs
    ///
    pub fn create_shader_loader() -> impl Send+Fn(StandardShaderProgram) -> ShaderProgram<ShaderUniform> {
        // Load the GLSL programs into memory
        let simple_vertex                       = String::from_utf8(include_bytes!["../../shaders/simple/simple.glslv"].to_vec()).unwrap();
        let simple_fragment                     = String::from_utf8(include_bytes!["../../shaders/simple/simple.glslf"].to_vec()).unwrap();
        let dashed_line_fragment                = String::from_utf8(include_bytes!["../../shaders/dashed_line/dashed_line.glslf"].to_vec()).unwrap();
        let texture_vertex                      = String::from_utf8(include_bytes!["../../shaders/texture/texture.glslv"].to_vec()).unwrap();
        let texture_fragment                    = String::from_utf8(include_bytes!["../../shaders/texture/texture.glslf"].to_vec()).unwrap();
        let gradient_vertex                     = String::from_utf8(include_bytes!["../../shaders/texture/gradient.glslv"].to_vec()).unwrap();
        let gradient_fragment                   = String::from_utf8(include_bytes!["../../shaders/texture/gradient.glslf"].to_vec()).unwrap();
        let msaa_vertex                         = String::from_utf8(include_bytes!["../../shaders/simple/resolve.glslv"].to_vec()).unwrap();
        let msaa4_resolve                       = String::from_utf8(include_bytes!["../../shaders/simple/multisample_resolve_4.glslf"].to_vec()).unwrap();

        // Incorporate them into the shader loader function
        move |program_type| {
            use StandardShaderProgram::*;

            match program_type {
                Simple(variant, post_process)               => { Self::load_shader(&simple_vertex, &vec!["a_Pos", "a_Color", "a_TexCoord"], &simple_fragment, &vec![], &variant.defines().into_iter().chain(post_process.defines()).collect()) }
                Texture(variant, post_process)              => { Self::load_shader(&texture_vertex, &vec!["a_Pos", "a_Color", "a_TexCoord"], &texture_fragment, &vec![], &variant.defines().into_iter().chain(post_process.defines()).collect()) }
                LinearGradient(variant, post_process)       => { Self::load_shader(&gradient_vertex, &vec!["a_Pos", "a_Color", "a_TexCoord"], &gradient_fragment, &vec![], &variant.defines().into_iter().chain(post_process.defines()).collect()) }
                DashedLine(variant, post_process)           => { Self::load_shader(&simple_vertex, &vec!["a_Pos", "a_Color", "a_TexCoord"], &dashed_line_fragment, &vec![], &variant.defines().into_iter().chain(post_process.defines()).collect()) }

                MsaaResolve(4, post_process)                => { Self::load_shader(&msaa_vertex, &vec![], &msaa4_resolve, &vec![], &post_process.defines()) }
                MsaaResolve(_num_samples, _post_process)    => { unimplemented!() }
            }
        }
    }
}

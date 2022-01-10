use super::shader::*;
use super::texture::*;
use super::shader_program::*;

use gl;

use std::hash::{Hash};

///
/// Every shader used by the renderer has four variants: 'basic', 'erase', 'clip' and 'erase/clip':
/// these correspond to the shaders that do or do not take input from the two masking textures.
///
/// These 4 programs are compiled by taking a shader program and amending it with different
/// #defines: this way only a single shader program can be used to produce all 4 variants.
///
pub struct ShaderCollection<UniformAttribute>
where UniformAttribute: Hash+Eq {
    pub basic:      ShaderProgram<UniformAttribute>,
    pub clip:       ShaderProgram<UniformAttribute>,
}

impl<UniformAttribute> ShaderCollection<UniformAttribute>
where UniformAttribute: Hash+Eq {
    ///
    /// Creates a new shader collection from the specified vertex and fragment programs
    ///
    pub fn new(vertex_program: &str, vertex_attributes: Vec<&str>, fragment_program: &str, fragment_attributes: Vec<&str>) -> ShaderCollection<UniformAttribute> {
        let basic_vertex        = Self::compile(vertex_program,    &vertex_attributes,     GlShaderType::Vertex,   &vec![]);
        let basic_fragment      = Self::compile(fragment_program,  &fragment_attributes,   GlShaderType::Fragment, &vec![]);
        let basic               = ShaderProgram::from_shaders(vec![basic_vertex, basic_fragment]);

        let clip_vertex         = Self::compile(vertex_program,    &vertex_attributes,     GlShaderType::Vertex,   &vec!["CLIP_MASK"]);
        let clip_fragment       = Self::compile(fragment_program,  &fragment_attributes,   GlShaderType::Fragment, &vec!["CLIP_MASK"]);
        let clip                = ShaderProgram::from_shaders(vec![clip_vertex, clip_fragment]);

        ShaderCollection {
            basic,
            clip
        }
    }

    ///
    /// Compiles a shader program with a set of defines
    ///
    fn compile(program: &str, attributes: &Vec<&str>, shader_type: GlShaderType, defines: &Vec<&str>) -> Shader {
        let program = format!("{}\n\n{}\n{}\n", 
            "#version 330 core",
            defines.iter().map(|defn| format!("#define {}\n", defn)).collect::<Vec<_>>().join(""),
            program);

        Shader::compile(&program, shader_type, attributes.iter().map(|s| *s))
    }

    ///
    /// Uses the appropriate program for the specified textures
    ///
    /// Textures 1 and 2 are used for the erase and clip mask: texture 0 is intended as the shader input, but 3 and upwards can be used as
    /// well, provided care is taken if we ever need more 'standard' variants
    ///
    pub fn use_shader<'a>(&'a mut self, clip_uniform: UniformAttribute, clip_texture: Option<&Texture>) -> &'a mut ShaderProgram<UniformAttribute> {
        unsafe {
            // Pick the program based on the requested textures
            let program = match clip_texture.is_some() {
                false   => &mut self.basic,
                true    => &mut self.clip,
            };
            gl::UseProgram(**program);

            // Apply the textures
            if let Some(texture) = clip_texture {
                // Set the clip texture
                gl::ActiveTexture(gl::TEXTURE2);
                gl::BindTexture(gl::TEXTURE_2D_MULTISAMPLE, **texture);

                program.uniform_location(clip_uniform, "t_ClipMask")
                    .map(|clip_mask| {
                        gl::Uniform1i(clip_mask, 2);
                    });
            }

            program
        }
    }
}

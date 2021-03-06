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
    pub erase:      ShaderProgram<UniformAttribute>,
    pub clip:       ShaderProgram<UniformAttribute>,
    pub clip_erase: ShaderProgram<UniformAttribute>,
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

        let erase_vertex        = Self::compile(vertex_program,    &vertex_attributes,     GlShaderType::Vertex,   &vec!["ERASE_MASK"]);
        let erase_fragment      = Self::compile(fragment_program,  &fragment_attributes,   GlShaderType::Fragment, &vec!["ERASE_MASK"]);
        let erase               = ShaderProgram::from_shaders(vec![erase_vertex, erase_fragment]);

        let clip_vertex         = Self::compile(vertex_program,    &vertex_attributes,     GlShaderType::Vertex,   &vec!["CLIP_MASK"]);
        let clip_fragment       = Self::compile(fragment_program,  &fragment_attributes,   GlShaderType::Fragment, &vec!["CLIP_MASK"]);
        let clip                = ShaderProgram::from_shaders(vec![clip_vertex, clip_fragment]);

        let clip_erase_vertex   = Self::compile(vertex_program,    &vertex_attributes,     GlShaderType::Vertex,   &vec!["ERASE_MASK", "CLIP_MASK"]);
        let clip_erase_fragment = Self::compile(fragment_program,  &fragment_attributes,   GlShaderType::Fragment, &vec!["ERASE_MASK", "CLIP_MASK"]);
        let clip_erase          = ShaderProgram::from_shaders(vec![clip_erase_vertex, clip_erase_fragment]);

        ShaderCollection {
            basic,
            erase,
            clip,
            clip_erase
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
    pub fn use_shader(&mut self, erase_uniform: UniformAttribute, clip_uniform: UniformAttribute, erase_texture: Option<&Texture>, clip_texture: Option<&Texture>) {
        unsafe {
            // Pick the program based on the requested textures
            let program = match (erase_texture.is_some(), clip_texture.is_some()) {
                (false, false)  => &mut self.basic,
                (true, false)   => &mut self.erase,
                (false, true)   => &mut self.clip,
                (true, true)    => &mut self.clip_erase,
            };
            gl::UseProgram(**program);

            // Apply the textures
            if let Some(texture) = erase_texture {
                // Set the erase texture
                gl::ActiveTexture(gl::TEXTURE0);
                gl::BindTexture(gl::TEXTURE_2D_MULTISAMPLE, **texture);

                program.uniform_location(erase_uniform, "t_EraseMask")
                    .map(|erase_mask| {
                        gl::Uniform1i(erase_mask, 0);
                    });
            }

            if let Some(texture) = clip_texture {
                // Set the erase texture
                gl::ActiveTexture(gl::TEXTURE1);
                gl::BindTexture(gl::TEXTURE_2D_MULTISAMPLE, **texture);

                program.uniform_location(clip_uniform, "t_ClipMask")
                    .map(|clip_mask| {
                        gl::Uniform1i(clip_mask, 1);
                    });
            }
        }
    }
}

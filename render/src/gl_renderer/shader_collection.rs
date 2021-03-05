use super::shader::*;
use super::shader_program::*;

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
}
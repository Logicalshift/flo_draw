use super::shader_program::*;

use gl;

use std::hash::{Hash};
use std::collections::{HashMap};

///
/// Every shader used by the renderer has four variants: 'basic', 'erase', 'clip' and 'erase/clip':
/// these correspond to the shaders that do or do not take input from the two masking textures.
///
/// These 4 programs are compiled by taking a shader program and amending it with different
/// #defines: this way only a single shader program can be used to produce all 4 variants.
///
pub struct ShaderCollection<ShaderType, UniformAttribute>
where 
UniformAttribute:   Hash+Eq,
ShaderType:         Hash+Eq {
    /// The cached shaders for this collection
    shaders: HashMap<ShaderType, ShaderProgram<UniformAttribute>>,

    /// Loads the shader with the specified type
    load_shader: Box<dyn Send+Fn(ShaderType) -> ShaderProgram<UniformAttribute>>
}

impl<ShaderType, UniformAttribute> ShaderCollection<ShaderType, UniformAttribute>
where 
UniformAttribute:   Hash+Eq,
ShaderType:         Hash+Eq+Clone {
    ///
    /// Creates a new shader collection from the specified vertex and fragment programs
    ///
    pub fn new<ShaderLoader>(loader: ShaderLoader) -> ShaderCollection<ShaderType, UniformAttribute> 
    where ShaderLoader: 'static+Send+Fn(ShaderType) -> ShaderProgram<UniformAttribute> {
        ShaderCollection {
            shaders:        HashMap::new(),
            load_shader:    Box::new(loader)
        }
    }

    ///
    /// Retrieves the shader program with the specified type
    ///
    pub fn program<'a>(&'a mut self, shader_type: ShaderType) -> &'a mut ShaderProgram<UniformAttribute> {
        // Use the existing shader program, or compile a new one if this shader hasn't been used before
        let shaders     = &mut self.shaders;
        let load_shader = &self.load_shader;

        let program     = shaders.entry(shader_type.clone())
            .or_insert_with(move || (load_shader)(shader_type));
        program
    }

    ///
    /// Uses the appropriate program for the specified textures
    ///
    /// Textures 1 and 2 are used for the erase and clip mask: texture 0 is intended as the shader input, but 3 and upwards can be used as
    /// well, provided care is taken if we ever need more 'standard' variants
    ///
    pub fn use_program<'a>(&'a mut self, shader_type: ShaderType) -> &'a mut ShaderProgram<UniformAttribute> {
        unsafe {
            let program = self.program(shader_type);
            gl::UseProgram(**program);

            program
        }
    }
}

use super::shader::*;
use super::texture::*;

use gl;

use std::hash::{Hash};
use std::collections::{HashMap};
use std::ops::{Deref};
use std::ffi::{CString};

///
/// A shader program represents a combination of shaders that can be used to perform an actual drawing
///
pub struct ShaderProgram<UniformAttribute>
where UniformAttribute: Hash {
    /// The shader progam object
    shader_program: gl::types::GLuint,

    /// The shaders that make up the shader program
    _shaders: Vec<Shader>,

    /// The attributes for the shader program (indexed first by shader, then by attribute number)
    _attributes: Vec<Vec<gl::types::GLuint>>,

    /// The location of the known uniforms for this shader program
    uniform_attributes: HashMap<UniformAttribute, gl::types::GLint>
}

impl<UniformAttribute: Hash+Eq> ShaderProgram<UniformAttribute> {
    ///
    /// Creates a shader program from a list of shaders
    ///
    pub fn from_shaders<ShaderIter: IntoIterator<Item=Shader>>(shaders: ShaderIter) -> ShaderProgram<UniformAttribute> {
        unsafe {
            let shaders = shaders.into_iter().collect::<Vec<_>>();

            // Create the shader program
            let shader_program = gl::CreateProgram();

            // Attach the shaders
            for shader in shaders.iter() {
                gl::AttachShader(shader_program, **shader);
            }

            // Link the program
            gl::LinkProgram(shader_program);

            let mut success = 1;
            gl::GetProgramiv(shader_program, gl::LINK_STATUS, &mut success);
            if success == 0 {
                panic!("Failed to link shader program");
            }

            // Bind the attributes
            let mut next_attribute_id   = 0;
            let mut attributes          = vec![];

            for shader in shaders.iter() {
                let mut shader_attributes = vec![];

                for attribute_name in shader.attributes() {
                    // Store the mapping for this attribute
                    shader_attributes.push(next_attribute_id);

                    // Bind this attribute
                    gl::BindAttribLocation(shader_program, next_attribute_id, attribute_name.as_ptr());

                    next_attribute_id += 1;
                }

                attributes.push(shader_attributes);
            }

            // Generate the resulting shader program
            ShaderProgram {
                shader_program:     shader_program,
                _shaders:           shaders,
                _attributes:        attributes,
                uniform_attributes: HashMap::new()
            }
        }
    }

    ///
    /// Retrieves the location of a uniform variable for this progrma
    ///
    pub fn uniform_location(&mut self, uniform: UniformAttribute, uniform_name: &str) -> Option<gl::types::GLint> {
        let shader_program = self.shader_program;

        Some(*self.uniform_attributes
            .entry(uniform)
            .or_insert_with(|| {
                unsafe {
                    let name = CString::new(uniform_name).unwrap();
                    
                    gl::GetUniformLocation(shader_program, name.as_ptr())
                }
            }))
    }

    ///
    /// Assigns a texture to a particular texture unit and sets its index in a uniform
    ///
    pub fn use_texture(&mut self, uniform: UniformAttribute, uniform_name: &str, texture: &Texture, texture_num: u8) {
        unsafe {
            // Set the clip texture
            let texture_num = texture_num as gl::types::GLenum;
            gl::ActiveTexture(gl::TEXTURE0 + texture_num);
            gl::BindTexture(gl::TEXTURE_2D_MULTISAMPLE, **texture);

            self.uniform_location(uniform, uniform_name)
                .map(|clip_mask| {
                    gl::Uniform1i(clip_mask, 2);
                });
        }
    }
}

impl<UniformAttribute: Hash> Drop for ShaderProgram<UniformAttribute> {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.shader_program);
        }
    }
}

impl<UniformAttribute: Hash> Deref for ShaderProgram<UniformAttribute> {
    type Target = gl::types::GLuint;

    fn deref(&self) -> &gl::types::GLuint {
        &self.shader_program
    }
}

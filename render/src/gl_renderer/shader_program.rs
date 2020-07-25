use super::shader::*;

use gl;

use std::ops::{Deref};
use std::ffi::{CString};

///
/// A shader program represents a combination of shaders that can be used to perform an actual drawing
///
pub struct ShaderProgram {
    /// The shader progam object
    shader_program: gl::types::GLuint,

    /// The shaders that make up the shader program
    shaders: Vec<Shader>,

    /// The attributes for the shader program (indexed first by shader, then by attribute number)
    attributes: Vec<Vec<gl::types::GLuint>>
}

impl ShaderProgram {
    ///
    /// Creates a shader program from a list of shaders
    ///
    pub fn from_shaders<ShaderIter: IntoIterator<Item=Shader>>(shaders: ShaderIter) -> ShaderProgram {
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
                shader_program: shader_program,
                shaders:        shaders,
                attributes:     attributes
            }
        }
    }

    ///
    /// Given a shader number (offset into the shader iterator used to create the program) and an attribute number
    /// (offset into the attribute iterator supplied when creating the shader), retrieves the attribute ID for this program
    ///
    pub fn attribute_id(&self, shader_num: usize, attribute_num: usize) -> gl::types::GLuint {
        self.attributes[shader_num][attribute_num]
    }

    ///
    /// Finds the attribute with the specified name from the shaders in this program
    ///
    pub fn attribute_with_name(&self, attribute_name: &str) -> Option<gl::types::GLuint> {
        // Convert the name to a c-string
        let name = CString::new(attribute_name).ok()?;

        // Iterate through the attributes until we find one with a matching name
        for (shader_num, shader) in self.shaders.iter().enumerate() {
            for (attribute_num, attribute_name) in shader.attributes().iter().enumerate() {
                if attribute_name == &name {
                    return Some(self.attributes[shader_num][attribute_num]);
                }
            }
        }

        None
    }
}

impl Drop for ShaderProgram {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.shader_program);
        }
    }
}

impl Deref for ShaderProgram {
    type Target = gl::types::GLuint;

    fn deref(&self) -> &gl::types::GLuint {
        &self.shader_program
    }
}

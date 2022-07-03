use wgpu;

use std::sync::*;
use std::hash::{Hash};
use std::collections::{HashMap};

///
/// Trait implemented by types that can be converted to a shader
///
pub trait WgpuShaderLoader {
    /// Loads a shader using this definition, returning the shader module and the vertex and fragment shader entry points
    fn load(&self, device: &wgpu::Device) -> (Arc<wgpu::ShaderModule>, String, String);
}

///
/// Caches the WGPU shaders so that they only need to be loaded once
///
pub struct ShaderCache<TShader>
where
    TShader: WgpuShaderLoader + Hash + Eq,
{
    /// The device that the shaders will be loaded on
    device: Arc<wgpu::Device>,

    /// The shaders stored in this cache
    shaders: HashMap<TShader, (Arc<wgpu::ShaderModule>, String, String)>
}

impl<TShader> ShaderCache<TShader>
where
    TShader: WgpuShaderLoader + Hash + Eq + Clone,
{
    ///
    /// Creates an empty shader cache
    ///
    pub fn empty(device: Arc<wgpu::Device>) -> ShaderCache<TShader> {
        ShaderCache {
            device:     device,
            shaders:    HashMap::new(),
        }
    }

    ///
    /// Loads the specified shader if it's not in the cache
    ///
    pub fn load_shader(&mut self, shader: &TShader) {
        if !self.shaders.contains_key(shader) {
            let new_shader = shader.load(&*self.device);
            self.shaders.insert(shader.clone(), new_shader);
        }
    }

    ///
    /// Retrieves the specified shader, if it's in the cache
    ///
    #[inline]
    pub fn get_shader<'a>(&'a self, shader: &TShader) -> Option<(&'a wgpu::ShaderModule, &'a str, &'a str)> {
        self.shaders.get(shader)
            .map(|(shader_ref, vertex_name, fragment_name)| {
                (&**shader_ref, vertex_name.as_str(), fragment_name.as_str())
            })
    }
}

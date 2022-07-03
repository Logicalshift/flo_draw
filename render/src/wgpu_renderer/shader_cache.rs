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
    TShader: WgpuShaderLoader + Hash + Eq,
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
    /// Retrieves the specified shader (caching it if it's not already in the cache)
    ///
    #[inline]
    pub fn get_shader(&mut self, shader: TShader) -> (Arc<wgpu::ShaderModule>, String, String) {
        if let Some(existing_shader) = self.shaders.get(&shader) {
            existing_shader.clone()
        } else {
            let new_shader = shader.load(&*self.device);
            self.shaders.insert(shader, new_shader.clone());

            new_shader
        }
    }
}

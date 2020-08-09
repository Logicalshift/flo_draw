use metal;

///
/// Renderer that can write to a surface using Apple's Metal API
///
pub struct MetalRenderer {
    device: metal::Device
}

impl MetalRenderer {
    ///
    /// Creates a new metal renderer using the system default device
    ///
    pub fn with_default_device() -> MetalRenderer {
        let device = metal::Device::system_default().expect("No Metal device available");

        MetalRenderer {
            device: device
        }
    }
}

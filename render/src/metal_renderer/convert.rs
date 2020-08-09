use super::bindings::*;
use crate::buffer::*;

use std::mem;

impl From<Vertex2D> for MetalVertex2D {
    fn from(src: Vertex2D) -> MetalVertex2D {
        unsafe {
            // The SIMD types do not come out in a very convenient form, so we use mem::transmute here
            MetalVertex2D {
                pos:        mem::transmute(src.pos),
                tex_coord:  mem::transmute(src.tex_coord),
                color:      mem::transmute(src.color)
            }
        }
    }
}
use crate::pixel_program::*;

use once_cell::sync::{Lazy};

use std::ops::{Range};
use std::sync::*;
use std::ptr;

///
/// The pixel program cache provides a way to assign IDs to pixel programs and support initialising them
/// with a data cache.
///
pub struct PixelProgramCache {

}

///
/// The pixel program data cache stores the program data for the pixel programs
///
pub struct PixelProgramDataCache {
    data: Vec<*mut ()>,
}

///
/// A data manager is used to store data associated with a program into a data cache
///
pub struct PixelProgramDataManager<TProgramData> {
    // Write the data for running this pixel program to the data cache 
    write_program_data: Box<dyn Fn(TProgramData, &mut PixelProgramDataCache) -> PixelProgramDataId>,
}

///
/// Identifier for the program data for a pixel program
///
/// Every pixel program has a separate set of identifiers for their data
///
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PixelProgramDataId(usize);

///
/// Identifier for some scanline data
///
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PixelScanlineDataId(usize);

impl PixelProgramDataCache {
    ///
    /// Unsafely find or create the program data cache for a pixel program
    ///
    /// Safety: `TProgramData` must be the same if the `program_id` is the same
    ///
    unsafe fn get_program_data_mut_unchecked<TProgramData>(&mut self, PixelProgramId(program_id): PixelProgramId) -> &mut Vec<TProgramData> {
        // Ensure enough space
        while self.data.len() <= program_id {
            self.data.push(ptr::null_mut());
        }

        // Allocate the vec if necessary

        // .. a plan B might be an 'instantiate' function that takes the program and some data and creates a function that takes a scanline iterator, finally returning a
        // function that can be called on each scanline (this avoids messing with pointer casts)

        todo!()
    }
}

impl PixelProgramCache {
    ///
    /// Creates an empty pixel program cache
    ///
    pub fn empty() -> PixelProgramCache {
        PixelProgramCache {
        }
    }

    ///
    /// Creates a function based on a program that sets its data and scanline data, generating the 'make pixels at position' function
    ///
    fn create_set_program_data<TProgram>(program: Arc<TProgram>) -> impl Fn(TProgram::ProgramData) -> Box<dyn Fn(i32, &Vec<PixelProgramScanline>) -> Box<dyn Fn(&mut [[f32; 4]], Range<i32>, i32) ->() >>
    where
        TProgram: 'static + PixelProgram,
    {
        move |program_data| {
            // Copy the program
            let program         = Arc::clone(&program);
            let program_data    = Arc::new(program_data);

            // Return a function that takes the scanlines and returns the rendering function
            Box::new(move |min_y, scanlines| {
                let scanline_data   = program.create_scanline_data(min_y, scanlines, &*program_data);
                let program         = Arc::clone(&program);
                let program_data    = Arc::clone(&program_data);

                Box::new(move |target, x_range, y_pos| {
                    program.draw_pixels(target, x_range, y_pos, &*program_data, &scanline_data)
                })
            })
        }
    }

    ///
    /// Caches a pixel program, returns its ID and a data manager to store data relating to the program
    ///
    pub fn add_program<TProgram>(&mut self, program: TProgram) -> (PixelProgramId, PixelProgramDataManager<TProgram::ProgramData>) 
    where
        TProgram: 'static + PixelProgram,
    {
        static NEXT_PROGRAM_ID: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(0));

        // Assign a data cache index for this program (or this program's data type? Might be easier to just make it per-program though)
        let new_program_id = {
            let mut next_program_id = NEXT_PROGRAM_ID.lock().unwrap();
            let new_program_id      = *next_program_id;
            *next_program_id        += 1;

            new_program_id
        };
        let new_program_id = PixelProgramId(new_program_id);

        todo!()
    }

    ///
    /// Creates scanline data for a program 
    ///
    pub fn create_scanline_data(&self, data_cache: &mut PixelProgramDataCache, program_id: PixelProgramId, x_range: Range<f32>, ypos: i32, program_data: PixelProgramDataId) -> PixelScanlineDataId {
        todo!()        
    }

    ///
    /// Runs a program on a range of pixels
    ///
    pub fn run_program(&self, data_cache: &PixelProgramDataCache, program_id: PixelProgramId, target: &mut [[f32; 4]], x_range: Range<i32>, ypos: i32, program_data: PixelProgramDataId, scanline_data: PixelScanlineDataId) {
        todo!()
    }
}

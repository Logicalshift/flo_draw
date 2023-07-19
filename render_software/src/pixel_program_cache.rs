use crate::pixel_program::*;

use once_cell::sync::{Lazy};

use std::marker::{PhantomData};
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
    /// Creates the 'write program data' function
    ///
    fn create_write_program_data<TProgram>(program: Arc<TProgram>, program_id: PixelProgramId) -> impl for<'a> Fn(TProgram::ProgramData, &'a mut PixelProgramDataCache) -> PixelProgramDataId 
    where
        TProgram: PixelProgram,
    {
        move |program_data, data_cache| {
            PixelProgramDataId(0)
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

        // Convert the program to read from the data cache
        let program_1 = Arc::new(program);
        let program_2 = Arc::clone(&program_1);
        let program_3 = Arc::clone(&program_2);

        // Create the data manager
        let data_manager = PixelProgramDataManager {
            write_program_data: Box::new(Self::create_write_program_data(program_1, new_program_id))
        };

        // Store the program in the cache
        (new_program_id, data_manager)
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

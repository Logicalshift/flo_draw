use crate::pixel_program::*;

use std::ops::{Range};
use std::sync::*;

///
/// The pixel program cache provides a way to assign IDs to pixel programs and support initialising them
/// with a data cache.
///
pub struct PixelProgramCache {
    next_program_id: usize
}

///
/// The pixel program data cache stores the program data for the pixel programs
///
pub struct PixelProgramDataCache {
    /// Program data is encapsulated in a function that generates the scanline data. This is indexed by `PixelProgramDataId`
    program_data: Vec<Box<dyn Fn(i32, &Vec<PixelProgramScanline>) -> Box<dyn Fn(&mut [[f32; 4]], Range<i32>, i32) -> ()>>>,

    /// The scanline_data functions encapsulate the program data and the scanline data indicate programs that are ready to run
    scanline_data: Vec<Box<dyn Fn(&mut [[f32; 4]], Range<i32>, i32) -> ()>>,
}

///
/// A data manager is used to store data associated with a program into a data cache
///
pub struct StoredPixelProgram<TProgram>
where
    TProgram: 'static + PixelProgram,
{
    /// The ID of this pixel program
    program_id: PixelProgramId,

    /// Function to associate program data with this program
    associate_program_data: Box<dyn Fn(TProgram::ProgramData) -> Box<dyn Fn(i32, &Vec<PixelProgramScanline>) -> Box<dyn Fn(&mut [[f32; 4]], Range<i32>, i32) -> ()>>>,
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
}

impl PixelProgramCache {
    ///
    /// Creates an empty pixel program cache
    ///
    pub fn empty() -> PixelProgramCache {
        PixelProgramCache {
            next_program_id: 0
        }
    }

    ///
    /// Creates a function based on a program that sets its data and scanline data, generating the 'make pixels at position' function
    ///
    fn create_associate_program_data<TProgram>(program: Arc<TProgram>) -> impl Fn(TProgram::ProgramData) -> Box<dyn Fn(i32, &Vec<PixelProgramScanline>) -> Box<dyn Fn(&mut [[f32; 4]], Range<i32>, i32) -> ()>>
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
    /// Caches a pixel program, assigning it an ID, and a cache that can be used
    ///
    pub fn add_program<TProgram>(&mut self, program: TProgram) -> StoredPixelProgram<TProgram> 
    where
        TProgram: 'static + PixelProgram,
    {
        // Assign an ID to the new program
        let new_program_id = self.next_program_id;
        let new_program_id = PixelProgramId(new_program_id);

        self.next_program_id += 1;

        // Create the function to associate data with this program
        let associate_data = Box::new(Self::create_associate_program_data(Arc::new(program)));

        // Return a stored pixel program of the appropriate type
        StoredPixelProgram {
            program_id:             new_program_id,
            associate_program_data: associate_data,
        }
    }

    ///
    /// Creates scanline data for a program 
    ///
    pub fn create_scanline_data(&self, data_cache: &mut PixelProgramDataCache, program_id: PixelProgramId, min_y: i32, scanlines: &Vec<PixelProgramScanline>, program_data: PixelProgramDataId) -> PixelScanlineDataId {
        todo!()
    }

    ///
    /// Runs a program on a range of pixels
    ///
    pub fn run_program(&self, data_cache: &PixelProgramDataCache, program_id: PixelProgramId, target: &mut [[f32; 4]], x_range: Range<i32>, ypos: i32, program_data: PixelProgramDataId, scanline_data: PixelScanlineDataId) {
        todo!()
    }
}

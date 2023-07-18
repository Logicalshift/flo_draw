use crate::pixel_program::*;

use std::marker::{PhantomData};

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

}

///
/// A data manager is used to store data associated with a program into a data cache
///
pub struct PixelProgramDataManager<TProgramData> {
    data: PhantomData<TProgramData>
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
    /// Caches a pixel program, returns its ID and a data manager to store data relating to the program
    ///
    pub fn add_program<TProgram>(&mut self, program: TProgram) -> (PixelProgramId, PixelProgramDataManager<TProgram::ProgramData>) 
    where
        TProgram: PixelProgram,
    {
        todo!()
    }
}
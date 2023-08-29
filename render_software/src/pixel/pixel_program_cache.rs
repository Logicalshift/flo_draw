use super::pixel_program::*;
use super::pixel_program_runner::*;

use std::marker::{PhantomData};
use std::ops::{Range};
use std::sync::*;

///
/// The pixel program cache provides a way to assign IDs to pixel programs and support initialising them
/// with a data cache.
///
pub struct PixelProgramCache<TPixel: Send> {
    next_program_id:    usize,
    phantom_data:       PhantomData<Mutex<TPixel>>,
}

///
/// The pixel program data cache stores the program data for the pixel programs
///
pub struct PixelProgramDataCache<TPixel: Send> {
    /// Functions that call a pixel program with its associated program data
    program_data: Vec<Box<dyn Send + Sync + Fn(&PixelProgramDataCache<TPixel>, &mut [TPixel], Range<i32>, f64) -> ()>>,

    /// The number of times each program_data item is used (0 when free)
    retain_counts: Vec<usize>,

    /// Slots in the 'program_data' list that are available to re-use with different data
    free_data_slots: Vec<usize>,
}

///
/// A stored pixel program can be used with a `PixelProgramDataCache` to save data to be used with pixel programs
///
pub struct StoredPixelProgram<TProgram>
where
    TProgram: 'static + PixelProgram,
{
    /// The ID of this pixel program
    program_id: PixelProgramId,

    /// Function to associate program data with this program
    associate_program_data: Box<dyn Fn(TProgram::ProgramData) -> Box<dyn Send + Sync + Fn(&PixelProgramDataCache<TProgram::Pixel>, &mut [TProgram::Pixel], Range<i32>, f64) -> ()>>,
}

///
/// Identifier for the program data for a pixel program
///
/// Every pixel program has a separate set of identifiers for their data
///
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct PixelProgramDataId(pub usize);

impl<TProgram> StoredPixelProgram<TProgram>
where
    TProgram: 'static + PixelProgram,
{
    /// Returns the program ID of this program
    #[inline]
    pub fn program_id(&self) -> PixelProgramId {
        self.program_id
    }
}

impl<TPixel> PixelProgramCache<TPixel>
where
    TPixel: 'static + Send,
{
    ///
    /// Creates an empty pixel program cache
    ///
    pub fn empty() -> PixelProgramCache<TPixel> {
        PixelProgramCache {
            next_program_id:    0,
            phantom_data:       PhantomData,
        }
    }

    ///
    /// Creates a function based on a program that sets its data and scanline data, generating the 'make pixels at position' function
    ///
    fn create_associate_program_data<TProgram>(program: Arc<TProgram>) -> impl Send + Sync + Fn(TProgram::ProgramData) -> Box<dyn Send + Sync + Fn(&PixelProgramDataCache<TPixel>, &mut [TPixel], Range<i32>, f64) -> ()>
    where
        TProgram: 'static + PixelProgram<Pixel=TPixel>,
    {
        move |program_data| {
            // Copy the program
            let program         = Arc::clone(&program);

            // Return a function that encapsulates the program data
            Box::new(move |data_cache, target, x_range, y_pos| {
                program.draw_pixels(data_cache, target, x_range, y_pos, &program_data)
            })
        }
    }

    ///
    /// Caches a pixel program, assigning it an ID, and a cache that can be used
    ///
    pub fn add_program<TProgram>(&mut self, program: TProgram) -> StoredPixelProgram<TProgram> 
    where
        TProgram: 'static + PixelProgram<Pixel=TPixel>,
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
    /// Creates a data cache to store data for rendering a frame with this pixel program cache
    ///
    pub fn create_data_cache(&mut self) -> PixelProgramDataCache<TPixel> {
        PixelProgramDataCache {
            program_data:       vec![],
            free_data_slots:    vec![],
            retain_counts:      vec![],
        }
    }

    ///
    /// Stores data to be used with an instance of a pixel program
    ///
    /// Program data can be a number of things: in the simplest case it might be the colour that the program will set the pixels to.
    /// `release_program_data()` can be used to free this data and make the ID available for reallocation to a different program. 
    ///
    pub fn store_program_data<TProgram>(&mut self, stored_program: &StoredPixelProgram<TProgram>, data_cache: &mut PixelProgramDataCache<TPixel>, data: TProgram::ProgramData) -> PixelProgramDataId 
    where
        TProgram: 'static + PixelProgram<Pixel=TPixel>,
    {
        // Generate the data for this program (well, encapsulate it in a function waiting for the scanline data)
        let associate_scanline_data = (stored_program.associate_program_data)(data);

        // Store in the data cache
        if let Some(program_data_id) = data_cache.free_data_slots.pop() {
            // Overwrite the program data in the unused slot
            data_cache.program_data[program_data_id]  = associate_scanline_data;
            data_cache.retain_counts[program_data_id] = 1;

            PixelProgramDataId(program_data_id)
        } else {
            // Assign an ID to this program data
            let program_data_id = data_cache.program_data.len();

            // Store the data in the cache
            data_cache.program_data.push(associate_scanline_data);
            data_cache.retain_counts.push(1);

            PixelProgramDataId(program_data_id)
        }
    }

    ///
    /// Increase the retain count for the specified program data ID
    ///
    /// Pixel program data will only be freed if release is called for every time this is called, plus once more for the initial allocation.
    ///
    #[inline]
    pub fn retain_program_data(&mut self, data_cache: &mut PixelProgramDataCache<TPixel>, data_id: PixelProgramDataId) {
        data_cache.retain_counts[data_id.0] += 1;
    }

    ///
    /// Increase the retain count for the specified program data ID
    ///
    /// Pixel program data will only be freed if release is called for every time this is called, plus once more for the initial allocation.
    ///
    #[inline]
    pub fn release_program_data(&mut self, data_cache: &mut PixelProgramDataCache<TPixel>, data_id: PixelProgramDataId) {
        if data_cache.retain_counts[data_id.0] == 0 {
            // Already freed
        } else if data_cache.retain_counts[data_id.0] == 1 {
            // Free the data for this program
            data_cache.retain_counts[data_id.0] = 0;
            data_cache.program_data[data_id.0]  = Box::new(|_, _, _, _| { });
            data_cache.free_data_slots.push(data_id.0);
        } else {
            // Reduce the retain count
            data_cache.retain_counts[data_id.0] -= 1;
        }
    }

    ///
    /// Frees all of the program data in a data cache, regardless of usage count
    ///
    pub fn free_all_data(&mut self, data_cache: &mut PixelProgramDataCache<TPixel>) {
        data_cache.free_data_slots.clear();
        data_cache.retain_counts.clear();
        data_cache.program_data.clear();
    }
}

impl<TPixel> PixelProgramRunner for PixelProgramDataCache<TPixel>
where
    TPixel: Send,
{
    type TPixel = TPixel;

    ///
    /// Runs a program on a range of pixels
    ///
    #[inline]
    fn run_program(&self, program_data: PixelProgramDataId, target: &mut [TPixel], x_range: Range<i32>, y_pos: f64) {
        (self.program_data[program_data.0])(self, target, x_range, y_pos)
    }
}

impl<'a, TPixel> PixelProgramRunner for &'a PixelProgramDataCache<TPixel> 
where
    TPixel: Send,
{
    type TPixel = TPixel;

    ///
    /// Runs a program on a range of pixels
    ///
    #[inline]
    fn run_program(&self, program_data: PixelProgramDataId, target: &mut [TPixel], x_range: Range<i32>, y_pos: f64) {
        (self.program_data[program_data.0])(self, target, x_range, y_pos)
    }
}

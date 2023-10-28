use super::pixel_program::*;
use super::pixel_program_runner::*;

use crate::scanplan::*;

use std::marker::{PhantomData};
use std::ops::{Range};
use std::sync::*;

///
/// f64 value representing the size of a pixel in render units
///
#[derive(Copy, Clone, Debug)]
pub struct PixelSize(pub f64);

///
/// Definition of a dynamic function that is passed a pixel range and fills it in
///
/// (This is essentially a fragment shader that runs on the CPU)
///
pub type PixelProgramFn<'a, TPixel> = Box<dyn 'a + Send + Sync + Fn(&PixelProgramRenderCache<TPixel>, &mut [TPixel], Range<i32>, &ScanlineTransform, f64) -> ()>;

///
/// Function that binds a pixel program to a particular set of canvas properties
///
/// This can be used to pre-compute some data related to a pixel program, or select the algorithm most appropriate for a given
/// rendering operation (for example, the texture pixel program can choose to use mip-mapping to scale down a texture at this point)
///
/// This is generally useful for avoiding having to re-make decisions regarding the canvas every time a pixel program is invoked.
///
// TODO: passing in the data here is necessary for the lifetime, but this would be much simpler if it were possible to specify that
// the function was borrowed for the lifetime of the PixelProgramFn (so the data can be entirely elided)
type PixelRenderBindFn<TPixel> = Box<dyn Send + Sync + Fn(PixelSize) -> PixelProgramFn<'static, TPixel>>;

///
/// Function that creates a pixel program function by binding some per-scene data into it
///
type PixelProgramBindFn<TData, TPixel> = Box<dyn Send + Sync + Fn(TData) -> PixelRenderBindFn<TPixel>>;

///
/// The pixel program cache assigns IDs to pixel programs.
///
/// This can also be used to generate a data cache, which associates program-specific data to a pixel program (for example, the color or texture
/// data to use). The data cache can in turn generate a program runner, which can be used to run the programs to a buffer with a particular
/// set of settings.
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
    program_data: Vec<PixelRenderBindFn<TPixel>>,

    /// The number of times each program_data item is used (0 when free)
    retain_counts: Vec<usize>,

    /// Slots in the 'program_data' list that are available to re-use with different data
    free_data_slots: Vec<usize>,
}

///
/// The render cache contains the programs that will be run to render a frame using particular settings
///
pub struct PixelProgramRenderCache<'a, TPixel: Send> {
    /// Functions that call a pixel program with its associated program data
    program_data: Vec<PixelProgramFn<'a, TPixel>>,
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
    associate_program_data: PixelProgramBindFn<TProgram::ProgramData, TProgram::Pixel>,
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
    fn create_associate_program_data<TProgram>(program: Arc<TProgram>) -> impl Send + Sync + Fn(TProgram::ProgramData) -> PixelRenderBindFn<TPixel>
    where
        TProgram: 'static + PixelProgram<Pixel=TPixel>,
    {
        move |program_data| {
            // Copy the program
            let program         = Arc::clone(&program);
            let program_data    = Arc::new(program_data);

            // Return a function that encapsulates the program data
            Box::new(move |_pixel_size| {
                let program         = Arc::clone(&program);
                let program_data    = Arc::clone(&program_data);

                Box::new(move |data_cache, target, x_range, x_transform, y_pos| {
                    program.draw_pixels(data_cache, target, x_range, x_transform, y_pos, &*program_data)
                })
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
    pub fn store_program_data<TProgram>(&self, stored_program: &StoredPixelProgram<TProgram>, data_cache: &mut PixelProgramDataCache<TPixel>, data: TProgram::ProgramData) -> PixelProgramDataId 
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
}

impl<TPixel> PixelProgramDataCache<TPixel> 
where
    TPixel: Send,
{
    ///
    /// Increase the retain count for the specified program data ID
    ///
    /// Pixel program data will only be freed if release is called for every time this is called, plus once more for the initial allocation.
    ///
    #[inline]
    pub fn retain_program_data(&mut self, data_id: PixelProgramDataId) {
        self.retain_counts[data_id.0] += 1;
    }

    ///
    /// Increase the retain count for the specified program data ID
    ///
    /// Pixel program data will only be freed if release is called for every time this is called, plus once more for the initial allocation.
    ///
    #[inline]
    pub fn release_program_data(&mut self, data_id: PixelProgramDataId) {
        if self.retain_counts[data_id.0] == 0 {
            // Already freed
        } else if self.retain_counts[data_id.0] == 1 {
            // Free the data for this program
            self.retain_counts[data_id.0] = 0;
            self.program_data[data_id.0]  = Box::new(|_| { Box::new(|_, _, _, _, _| { }) });
            self.free_data_slots.push(data_id.0);
        } else {
            // Reduce the retain count
            self.retain_counts[data_id.0] -= 1;
        }
    }

    ///
    /// Frees all of the program data in a data cache, regardless of usage count
    ///
    pub fn free_all_data(&mut self) {
        self.free_data_slots.clear();
        self.retain_counts.clear();
        self.program_data.clear();
    }

    ///
    /// Initialises a render cache from this data cache. This 
    ///
    pub fn create_program_runner<'a>(&'a self, pixel_size: PixelSize) -> impl 'a + PixelProgramRunner<TPixel = TPixel> {
        PixelProgramRenderCache {
            program_data: self.program_data.iter().map(|data| data(pixel_size)).collect()
        }
    }
}

impl<'a, TPixel> PixelProgramRunner for PixelProgramRenderCache<'a, TPixel>
where
    TPixel: Send,
{
    type TPixel = TPixel;

    ///
    /// Runs a program on a range of pixels
    ///
    #[inline]
    fn run_program(&self, program_data: PixelProgramDataId, target: &mut [TPixel], x_range: Range<i32>, x_transform: &ScanlineTransform, y_pos: f64) {
        (self.program_data[program_data.0])(self, target, x_range, x_transform, y_pos)
    }
}

impl<'a, TPixel> PixelProgramRunner for &'a PixelProgramRenderCache<'a, TPixel> 
where
    TPixel: Send,
{
    type TPixel = TPixel;

    ///
    /// Runs a program on a range of pixels
    ///
    #[inline]
    fn run_program(&self, program_data: PixelProgramDataId, target: &mut [TPixel], x_range: Range<i32>, x_transform: &ScanlineTransform, y_pos: f64) {
        (self.program_data[program_data.0])(self, target, x_range, x_transform, y_pos)
    }
}

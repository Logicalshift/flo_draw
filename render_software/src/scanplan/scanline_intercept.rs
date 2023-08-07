use crate::edgeplan::*;

///
/// Represents an active intercept on a scanline
///
pub struct ScanlineIntercept<'a> {
    /// The number of times an edge for this shape has been crossed
    count: isize,

    /// The x-position where the shape was first intercepted
    start_x: f64,

    /// The Z-index of this scanline (scanlines are stored ordered by z-index then shape ID)
    z_index: i64,

    /// The shape that is being drawn by this scanline
    shape_id: ShapeId,

    /// The shape descriptor
    descriptor: &'a ShapeDescriptor,

    /// Opaque spans form the 'z-floor': spans below this point are not rendered
    is_opaque: bool,
}

///
/// Used to keep track of which shapes are being rendered when tracing a scanline
///
pub struct ScanlineInterceptState<'a> {
    /// The currently active shapes, with the most recent one 
    active_shapes: Vec<ScanlineIntercept<'a>>,

    /// The current z-floor
    z_floor: i64,
}

impl<'a> ScanlineIntercept<'a> {
    ///
    /// Returns the point at which this intercept started
    ///
    #[inline]
    pub fn start_x(&self) -> f64 {
        self.start_x
    }

    ///
    /// Returns the shape ID used for this intercept
    ///
    #[inline]
    pub fn shape_id(&self) -> ShapeId {
        self.shape_id
    }

    ///
    /// Returns true if this intercept is opaque
    ///
    #[inline]
    pub fn is_opaque(&self) -> bool {
        self.is_opaque
    }

    ///
    /// Returns the shape descriptor for this intercept
    ///
    #[inline]
    pub fn shape_descriptor(&self) -> &ShapeDescriptor {
        self.descriptor
    }
}

impl<'a> ScanlineInterceptState<'a> {
    ///
    /// Creates a new intercept state
    ///
    #[inline]
    pub fn new() -> ScanlineInterceptState<'a> {
        ScanlineInterceptState { 
            active_shapes:  vec![],
            z_floor:        i64::MIN,
        }
    }

    ///
    /// The z-index of the lowest opaque item in this state (or `i64::MIN` if there's no floor)
    ///
    #[inline]
    pub fn z_floor(&self) -> i64 { 
        self.z_floor
    }

    ///
    /// Finds the index of the intercept that's >= the z-index
    ///
    /// Returns Ok(index) if we find an exact match, or Err(index) if we don't
    ///
    #[inline]
    pub fn find(&self, z_index: i64, shape_id: ShapeId) -> Result<usize, usize> {
        // min is inclusive, max is exclusive
        let mut min = 0;
        let mut max = self.active_shapes.len();

        // Binary search until we find a nearby shape
        while min < max {
            let mid         = (min + max) >> 1;
            let intercept   = &self.active_shapes[mid];

            if intercept.z_index < z_index {
                min = mid + 1;
            } else if intercept.z_index > z_index {
                max = mid;
            } else if intercept.shape_id < shape_id {
                min = mid + 1;
            } else if intercept.shape_id > shape_id {
                max = mid;
            } else {
                return Ok(mid);
            }
        }

        /* (may be faster)
        // Linear search for the remaining items
        while min < max {
            let intercept = &self.active_shapes[min];

            if intercept.z_index > z_index {
                return Err(min);
            } else if intercept.shape_id > shape_id {
                return Err(min);
            } else if intercept.z_index == z_index && intercept.shape_id == shape_id {
                return Ok(min);
            }

            min += 1;
        }
        */

        // 'min' should be the first >= value once the binary search converges
        return Err(min);
    }

    ///
    /// The number of intercepts that are currently on the stack
    ///
    #[inline]
    pub fn len(&self) -> usize {
        self.active_shapes.len()
    }

    ///
    /// Retrieves the intercept at the specified position on the stack
    ///
    #[inline]
    pub fn get(&self, idx: usize) -> Option<&ScanlineIntercept> {
        self.active_shapes.get(idx)
    }

    ///
    /// Adds or removes from the active shapes after an intercept
    ///
    #[inline]
    pub fn add_intercept(&mut self, direction: EdgeInterceptDirection, shape_id: ShapeId, descriptor: Option<&'a ShapeDescriptor>, x_pos: f64) {
        if let Some(descriptor) = descriptor {
            let (z_index, is_opaque) = (descriptor.z_index, descriptor.is_opaque);

            match self.find(z_index, shape_id) {
                Ok(existing_idx) => {
                    // Update the existing shape depending on the direction of the intercept
                    let existing        = &mut self.active_shapes[existing_idx];
                    let remove_existing = match direction {
                        EdgeInterceptDirection::Toggle          => true,

                        EdgeInterceptDirection::DirectionOut    => {
                            existing.count += 1;
                            existing.count == 0
                        },

                        EdgeInterceptDirection::DirectionIn     => {
                            existing.count -= 1;
                            existing.count == 0
                        },
                    };

                    if remove_existing {
                        // If the count is 0 (or the edge is a toggle edge), then stop intercepting this shape
                        self.active_shapes.remove(existing_idx);

                        // If the shape matches the current z-floor, update it
                        if is_opaque && z_index == self.z_floor {
                            self.z_floor = i64::MIN;

                            if existing_idx > 0 {
                                // TODO: if multiple shapes are on the same z-index, existing_idx might represent a shape below the 'true' z-floor (so this will set the floor too low)
                                for idx in (0..(existing_idx-1)).rev() {
                                    if self.active_shapes[idx].is_opaque {
                                        self.z_floor = self.active_shapes[idx].z_index;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }

                Err(following_idx) => {
                    // There's no existing matching shape: just insert a new intercept
                    let count = match direction {
                        EdgeInterceptDirection::Toggle          => 1,
                        EdgeInterceptDirection::DirectionOut    => 1,
                        EdgeInterceptDirection::DirectionIn     => -1,
                    };

                    // Opaque shapes update the z-floor (note that if an opaque shape has the same z-index as another shape, the z-floor is not enough to tell which is in front)
                    if is_opaque {
                        self.z_floor = self.z_floor.max(z_index);
                    }

                    self.active_shapes.insert(following_idx, ScanlineIntercept { 
                        count:      count, 
                        start_x:    x_pos, 
                        z_index:    z_index, 
                        shape_id:   shape_id,
                        is_opaque:  is_opaque,
                        descriptor: descriptor,
                    })
                }
            }
        }
    }

    ///
    /// Adjusts all the existing intercepts so that they have a specified start position (for clipping onto the left-hand side of the visible region)
    ///
    pub fn clip_start_x(&mut self, clip_x: f64) {
        for intercept in self.active_shapes.iter_mut() {
            intercept.start_x = clip_x;
        }
    }
}

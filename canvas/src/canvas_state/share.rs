use std::sync::*;

///
/// Canvas brush that can be modified or shared
///
#[derive(Clone, Debug)]
pub struct CanvasShared<T>(CanvasSharedValue<T>);

///
/// Canvas brush that can be modified or shared
///
#[derive(Clone, Debug)]
enum CanvasSharedValue<T> {
    Empty,
    Modified(T),
    Shared(Arc<T>),
}

impl<T> CanvasShared<T> 
where
    T : Clone,
{
    ///
    /// Creates a new shared value from the interior value
    ///
    #[inline]
    pub fn new(val: T) -> Self {
        Self(CanvasSharedValue::Modified(val))
    }

    ///
    /// Retrieves the inner value as a mutable object
    ///
    pub fn get_mut(&mut self) -> &mut T {
        use std::mem;

        // If the value is shared, then clone it to make it modifiable
        if let CanvasSharedValue::Shared(_) = &self.0 {
            // Swap in the empty value
            let mut val = CanvasSharedValue::Empty;
            mem::swap(&mut val, &mut self.0);

            // Unwrap or clone the arc
            let CanvasSharedValue::Shared(val) = val else { unreachable!() };
            self.0 = CanvasSharedValue::Modified(Arc::unwrap_or_clone(val));
        }

        // Should always be modifiable at this point
        let CanvasSharedValue::Modified(val) = &mut self.0 else { unreachable!() };
        val
    }

    ///
    /// Shares this value
    ///
    pub fn shared(&mut self) -> Self {
        use std::mem;

        match &self.0 {
            CanvasSharedValue::Empty        => unreachable!(),

            CanvasSharedValue::Shared(val)  => Self(CanvasSharedValue::Shared(Arc::clone(val))),
            CanvasSharedValue::Modified(_)  => {
                // Swap in the empty value
                let mut val = CanvasSharedValue::Empty;
                mem::swap(&mut val, &mut self.0);

                // Change to a shared value
                let CanvasSharedValue::Modified(val) = val else { unreachable!() };
                self.0 = CanvasSharedValue::Shared(Arc::new(val));

                let CanvasSharedValue::Shared(val) = &self.0 else { unreachable!() };
                Self(CanvasSharedValue::Shared(Arc::clone(val)))
            }
        }
    }
}

impl<T> Default for CanvasShared<T>
where
    T : Default,
{
    #[inline]
    fn default() -> Self {
        Self(CanvasSharedValue::Modified(T::default()))
    }
}

use std::ops::{Range};

///
/// A buffer stack is used to store a list of 
///
pub struct BufferStack<'a, TPixel> {
    /// The raw stack entry for this buffer stack
    first: &'a mut [TPixel],

    /// The items that have been pushed to the stack
    stack: Vec<Vec<TPixel>>,

    /// Previously used pixel buffers waiting for use
    ready_stack: Vec<Vec<TPixel>>,
}

impl<'a, TPixel> BufferStack<'a, TPixel> 
where
    TPixel: Default + Copy,
{
    ///
    /// Creates a new buffer stack
    ///
    #[inline]
    pub fn new(buffer: &'a mut [TPixel]) -> Self {
        BufferStack {
            first:          buffer,
            stack:          Vec::with_capacity(8),
            ready_stack:    Vec::with_capacity(8),
        }
    }

    ///
    /// Borrows the buffer inside this stack
    ///
    #[inline]
    pub fn buffer<'b>(&'b mut self) -> &'b mut [TPixel] {
        if let Some(last) = self.stack.last_mut() {
            last
        } else {
            self.first
        }
    }

    ///
    /// Allocates a new entry on the stack, by copying a range of bytes from the previous entry
    ///
    /// Each layer of the stack is the same length, but only the bytes in the range are relevant for the next layer
    ///
    #[inline]
    pub fn push_entry(&mut self, range: Range<usize>) {
        if let Some(mut new_entry) = self.ready_stack.pop() {
            // Copy into the new entry from the existing entry (we already know this is large enough when it was allocated)
            if let Some(last) = self.stack.last() {
                new_entry[range.clone()].copy_from_slice(&last[range]);
            } else {
                new_entry[range.clone()].copy_from_slice(&self.first[range]);
            }

            // Push to the top of the stack
            self.stack.push(new_entry);
        } else {
            // Create a new buffer by copying whatever was last in the list
            let mut new_entry = vec![TPixel::default(); self.first.len()];

            if let Some(last) = self.stack.last() {
                new_entry[range.clone()].copy_from_slice(&last[range]);
            } else {
                new_entry[range.clone()].copy_from_slice(&self.first[range]);
            }

            self.stack.push(new_entry);
        }
    }

    ///
    /// Pops an entry and blends it with the underlying entry using a callback function
    ///
    #[inline]
    pub fn pop_entry(&mut self, blend_pixels: impl FnOnce(&[TPixel], &mut [TPixel])) {
        if let Some(removed) = self.stack.pop() {
            // Blend with the lower layer
            if let Some(last) = self.stack.last_mut() {
                blend_pixels(&removed, last);
            } else {
                blend_pixels(&removed, self.first);
            }

            // Store in the ready stack so we can re-use this buffer later on
            self.ready_stack.push(removed);
        }
    }
}

use std::{cell::Cell, mem::ManuallyDrop, ops::Deref, ptr::NonNull};
use trace::Trace;

mod trace;

#[cfg(test)]
mod tests;

#[repr(C)]
struct GcInner<T: ?Sized + Trace> {
    ref_count: Cell<usize>,
    /// Flag: This value is accessed from root
    accessed: Cell<bool>,
    /// Flag: This value is zombie and should be deallocated
    dropped: Cell<bool>,
    value: ManuallyDrop<T>,
}

impl<T: ?Sized + Trace> GcInner<T> {
    fn drop_value(&mut self) {
        if !self.dropped.get() {
            self.dropped.set(true);
            unsafe {
                ManuallyDrop::drop(&mut self.value);
            };
        }
    }
}

pub struct Gc<T: Trace> {
    ptr: NonNull<GcInner<T>>,
}

impl<T: Trace> Clone for Gc<T> {
    fn clone(&self) -> Gc<T> {
        unsafe {
            self.ptr.as_ref().ref_count.update(|rc| rc + 1);
        }
        Gc { ptr: self.ptr }
    }
}

impl<T: Trace> Drop for Gc<T> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let inner = self.ptr.as_mut();
            if !inner.dropped.get() {
                inner.ref_count.update(|rc| rc - 1);
                if inner.ref_count.get() == 0 {
                    inner.drop_value();
                }
            }
        }
    }
}

impl<T: Trace> Deref for Gc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &self.ptr.as_ref().value }
    }
}

impl<T: Trace> Trace for Gc<T> {
    fn trace(&self) {
        unsafe {
            let inner = self.ptr.as_ref();
            if !inner.accessed.get() {
                inner.accessed.set(true);
                inner.value.trace();
            }
        }
    }
}

/// Virtual heap with garbage collection
pub struct GcHeap {
    values: Vec<NonNull<GcInner<dyn Trace>>>,
}

impl GcHeap {
    pub fn new() -> GcHeap {
        GcHeap { values: vec![] }
    }

    /// Allocate new garbage-collectible object on a virtual heap
    pub fn alloc<T: Trace + 'static>(&mut self, value: T) -> Gc<T> {
        let raw_thin = Box::leak(Box::new(GcInner {
            ref_count: Cell::new(1),
            accessed: Cell::new(false),
            dropped: Cell::new(false),
            value: ManuallyDrop::new(value),
        }));
        let non_null_thin = NonNull::from(raw_thin);
        let non_null_dyn: NonNull<GcInner<dyn Trace>> = non_null_thin;

        self.values.push(non_null_dyn);
        Gc { ptr: non_null_thin }
    }

    /// sweep stage of garbage collection
    pub fn sweep(&mut self) {
        self.values.retain_mut(|ptr| unsafe {
            let inner = ptr.as_mut();
            if inner.dropped.get() {
                // Inner value already dropped by refcounter in Gc::drop().
                // Do not call drop on value second time
                ptr.drop_in_place();
                //drop(Box::from_raw(ptr.as_ptr()));
                false
            } else if !inner.accessed.get() {
                // Value is not accessed from root.
                // Drop inner value and whole box.
                inner.drop_value();
                ptr.drop_in_place();
                //drop(Box::from_raw(ptr.as_ptr()));
                false
            } else {
                ptr.as_ref().accessed.set(false);
                true
            }
        });
    }

    pub fn dropped_count(&self) -> usize {
        self.values
            .iter()
            .filter(|&ptr| unsafe {
                let inner = ptr.as_ref();
                inner.dropped.get()
            })
            .count()
    }
}

impl Drop for GcHeap {
    fn drop(&mut self) {
        self.sweep();
        if !self.values.is_empty() {
            panic!("GcHeap is not empty when dropped")
        }
    }
}

impl Default for GcHeap {
    fn default() -> Self {
        Self::new()
    }
}

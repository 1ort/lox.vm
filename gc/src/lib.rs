use std::{
    cell::{Cell, UnsafeCell},
    mem::ManuallyDrop,
    ops::Deref,
    ptr::NonNull,
};
use trace::Trace;

mod trace;

#[cfg(test)]
mod tests;

#[repr(C)]
struct GcHeader {
    ref_count: Cell<usize>,
    accessed: Cell<bool>,
    dropped: Cell<bool>,
    drop_fn: unsafe fn(*mut Self) -> (),
    dealloc_fn: unsafe fn(*mut Self) -> (),
}

unsafe fn drop_inner_value<T: Trace>(header_ptr: *mut GcHeader) {
    unsafe {
        if !(*header_ptr).dropped.get() {
            (*header_ptr).dropped.set(true);
            let t_ptr = header_ptr.cast::<GcInner<T>>();
            ManuallyDrop::drop((*t_ptr).value.get().as_mut_unchecked())
        }
    }
}

unsafe fn drop_and_dealloc_gc_inner<T: Trace>(header_ptr: *mut GcHeader) {
    let t_ptr = header_ptr.cast::<GcInner<T>>();
    unsafe {
        drop(Box::from_raw(t_ptr));
    }
}

unsafe fn decrement_rc<T: Trace>(header_ptr: *mut GcHeader) {
    unsafe {
        if (*header_ptr).dropped.get() {
            return;
        }
        (*header_ptr).ref_count.update(|rc| rc - 1);
        if (*header_ptr).ref_count.get() == 0 {
            drop_inner_value::<T>(header_ptr);
        }
    }
}

unsafe fn increment_rc(header_ptr: *mut GcHeader) {
    unsafe {
        (*header_ptr).ref_count.update(|rc| rc + 1);
    }
}
#[repr(C)]
struct GcInner<T: Sized + Trace> {
    header: GcHeader,
    value: UnsafeCell<ManuallyDrop<T>>,
}

pub struct Gc<T: Trace> {
    ptr: NonNull<GcInner<T>>,
}

impl<T: Trace> Clone for Gc<T> {
    fn clone(&self) -> Gc<T> {
        unsafe {
            let header_ptr = self.ptr.as_ptr().cast::<GcHeader>();
            increment_rc(header_ptr);
        }
        Gc { ptr: self.ptr }
    }
}

impl<T: Trace> Drop for Gc<T> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let header_ptr = self.ptr.as_ptr().cast::<GcHeader>();
            decrement_rc::<T>(header_ptr);
        }
    }
}

impl<T: Trace> Deref for Gc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { (&*self.ptr.as_ref().value.get()).deref() }
    }
}

impl<T: Trace> Trace for Gc<T> {
    fn trace(&self) {
        unsafe {
            let inner = self.ptr.as_ref();
            if !inner.header.accessed.get() {
                inner.header.accessed.set(true);
                (&*inner.value.get()).deref().trace();
            }
        }
    }
}

pub struct GcHeap {
    values: Vec<NonNull<GcHeader>>,
}

impl GcHeap {
    pub fn new() -> GcHeap {
        GcHeap { values: vec![] }
    }

    pub fn alloc<T: Trace + 'static>(&mut self, value: T) -> Gc<T> {
        let raw_inner = Box::leak(Box::new(GcInner {
            header: GcHeader {
                ref_count: Cell::new(1),
                accessed: Cell::new(false),
                dropped: Cell::new(false),
                drop_fn: drop_inner_value::<T>,
                dealloc_fn: drop_and_dealloc_gc_inner::<T>,
            },
            value: UnsafeCell::new(ManuallyDrop::new(value)),
        }));

        let non_null_inner = NonNull::from(raw_inner);
        let non_null_header: NonNull<GcHeader> = non_null_inner.cast::<GcHeader>();
        self.values.push(non_null_header);
        Gc {
            ptr: non_null_inner,
        }
    }

    pub fn sweep(&mut self) {
        // value pointers and layouts to deallocate
        let mut to_dealloc: Vec<*mut GcHeader> = vec![];

        self.values.retain(|&ptr| unsafe {
            let ptr = ptr.as_ptr();
            if (*ptr).dropped.get() {
                to_dealloc.push(ptr);
                false
            } else if !(*ptr).accessed.get() {
                ((*ptr).drop_fn)(ptr);
                to_dealloc.push(ptr);
                false
            } else {
                (*ptr).accessed.set(false);
                true
            }
        });

        for ptr in to_dealloc {
            unsafe { ((*ptr).dealloc_fn)(ptr) }
        }
    }

    pub fn dropped_count(&self) -> usize {
        self.values
            .iter()
            .filter(|&ptr| unsafe { (*ptr.as_ptr()).dropped.get() })
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

use std::{
    alloc::{Layout, dealloc},
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
struct GcInner<T: ?Sized + Trace> {
    ref_count: Cell<usize>,
    accessed: Cell<bool>,
    dropped: Cell<bool>,
    value: UnsafeCell<ManuallyDrop<T>>,
}

impl<T: ?Sized + Trace> GcInner<T> {
    fn drop_value(&self) {
        if !self.dropped.get() {
            self.dropped.set(true);
            unsafe {
                ManuallyDrop::drop(self.value.get().as_mut_unchecked());
            }
        }
    }
}

pub struct Gc<T: Trace> {
    ptr: NonNull<GcInner<T>>,
}

impl<T: Trace> Clone for Gc<T> {
    fn clone(&self) -> Gc<T> {
        unsafe {
            let inner = self.ptr.as_ref();
            let rc = inner.ref_count.get();
            inner.ref_count.set(rc + 1);
        }
        Gc { ptr: self.ptr }
    }
}

impl<T: Trace> Drop for Gc<T> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let inner = self.ptr.as_ref();
            if !inner.dropped.get() {
                let rc = inner.ref_count.get();
                inner.ref_count.set(rc - 1);
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
        unsafe {
            // Получаем &ManuallyDrop<T> через UnsafeCell, затем через deref получаем &T
            (&*self.ptr.as_ref().value.get()).deref()
        }
    }
}

impl<T: Trace> Trace for Gc<T> {
    fn trace(&self) {
        unsafe {
            let inner = self.ptr.as_ref();
            if !inner.accessed.get() {
                inner.accessed.set(true);
                // Получаем доступ к значению через UnsafeCell
                (&*inner.value.get()).deref().trace();
            }
        }
    }
}

/// Виртуальная куча с сборщиком мусора
pub struct GcHeap {
    values: Vec<(NonNull<GcInner<dyn Trace>>, Layout)>,
}

impl GcHeap {
    pub fn new() -> GcHeap {
        GcHeap { values: vec![] }
    }

    /// Выделяет новый объект с сборкой мусора на виртуальной куче
    pub fn alloc<T: Trace + 'static>(&mut self, value: T) -> Gc<T> {
        let raw_thin = Box::leak(Box::new(GcInner {
            ref_count: Cell::new(1),
            accessed: Cell::new(false),
            dropped: Cell::new(false),
            value: UnsafeCell::new(ManuallyDrop::new(value)),
        }));

        let layout = Layout::for_value(raw_thin);

        let non_null_thin = NonNull::from(raw_thin);
        let non_null_dyn: NonNull<GcInner<dyn Trace>> = non_null_thin;

        self.values.push((non_null_dyn, layout));
        Gc { ptr: non_null_thin }
    }

    /// Этап sweep сборки мусора
    pub fn sweep(&mut self) {
        // value pointers and layouts to deallocate
        let mut to_dealloc: Vec<(*mut u8, Layout)> = vec![];

        self.values.retain(|(ptr, layout)| unsafe {
            let ptr = ptr.as_ptr();
            if (*ptr).dropped.get() {
                ptr.drop_in_place();
                to_dealloc.push((ptr as *mut u8, *layout));
                //dealloc(ptr as *mut u8, *layout);
                false
            } else if !(*ptr).accessed.get() {
                (*ptr).drop_value();
                ptr.drop_in_place();
                to_dealloc.push((ptr as *mut u8, *layout));
                //dealloc(ptr as *mut u8, *layout);
                false
            } else {
                (*ptr).accessed.set(false);
                true
            }
        });

        for (ptr, layout) in to_dealloc {
            unsafe {
                dealloc(ptr, layout);
            }
        }
    }

    pub fn dropped_count(&self) -> usize {
        self.values
            .iter()
            .filter(|(ptr, _)| unsafe {
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

use std::{
    cell::{Cell, UnsafeCell},
    mem::ManuallyDrop,
    ops::Deref,
    ptr::{self, NonNull},
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
    values: Vec<NonNull<GcInner<dyn Trace>>>,
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
        let non_null_thin = NonNull::from(raw_thin);
        let non_null_dyn: NonNull<GcInner<dyn Trace>> = non_null_thin;

        self.values.push(non_null_dyn);
        Gc { ptr: non_null_thin }
    }

    /// Этап sweep сборки мусора
    pub fn sweep(&mut self) {
        self.values.retain_mut(|ptr| unsafe {
            let inner = ptr.as_ref();
            if inner.dropped.get() {
                // Значение уже сброшено в Gc::drop, повторно не дропаем
                ptr.drop_in_place();
                false
            } else if !inner.accessed.get() {
                // Значение не достижимо из корня – дропаем и удаляем
                inner.drop_value();
                ptr.drop_in_place();
                false
            } else {
                // Сбрасываем флаг accessed для следующего цикла
                inner.accessed.set(false);
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


use std::{cell::Cell, ops::Deref, ptr::NonNull};
use trace::Trace;

mod trace;

/// Common header for every value on a GcHeap
pub struct GcHeader {
    /// Gc itself inherits reference counting semantics
    /// This field is a counter
    ref_count: Cell<usize>,
    /// Flag: This field is accessed from root
    accessed: Cell<bool>,
    dead: Cell<bool>,
    // TODO: merge flags and ref_counter into single field
}

impl GcHeader {
    fn ref_count(&self) -> usize {
        self.ref_count.get()
    }
    fn increment_rc(&self) {
        self.ref_count.update(|x| x + 1);
    }
    fn decrement_rc(&self) {
        self.ref_count.update(|x| x - 1);
    }
    fn is_accessed(&self) -> bool {
        self.accessed.get()
    }

    fn is_dead(&self) -> bool {
        self.dead.get()
    }
}

#[repr(C)]
struct GcInner<T: Trace> {
    header: GcHeader,
    /// None is only possible for zombie values
    value: Option<T>,
}

pub struct Gc<T: Trace> {
    ptr: NonNull<GcInner<T>>,
}

impl<T: Trace> Clone for Gc<T> {
    fn clone(&self) -> Gc<T> {
        // SAFETY: we ensure that no Gc will outlive it's GcInner
        unsafe {
            self.header().increment_rc();
        }
        Gc { ptr: self.ptr }
    }
}

impl<T: Trace> Drop for Gc<T> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            self.header().decrement_rc();
            if self.header().ref_count() == 0 {
                self.drop_value();
            }
        }
    }
}

impl<T: Trace> Deref for Gc<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        // SAFETY: it's garanteed that pointer is valid:
        // 1) Value was not dropped by ref counter (self is a valid Gc)
        // 2) GcHeap is responsible for cleansing during the sweep phase.
        unsafe {
            self.ptr
                .as_ref()
                .value
                .as_ref()
                .expect("Attempt to deref zombie value")
        }
    }
}

impl<T: Trace> Trace for Gc<T> {
    fn trace(&self) {
        unsafe {
            if !self.header().is_accessed() {
                self.header().accessed.set(true);
                self.deref().trace();
            }
        }
    }
}

impl<T: Trace> Gc<T> {
    unsafe fn header(&self) -> &GcHeader {
        // SAFETY: Caller must ensure that Gc is valid pointer
        unsafe { &self.ptr.as_ref().header }
    }

    /// Non-inlined part of drop.
    /// Drops inner value
    #[inline(never)]
    unsafe fn drop_value(&mut self) {
        unsafe {
            let value_ref = &mut (*self.ptr.as_ptr()).value;
            let _ = value_ref.take();
        }
    }
}

/// Virtual heap with garbage collection
pub struct GcHeap {
    headers: Vec<Option<NonNull<GcHeader>>>,
    free_slots: Vec<usize>,
}

impl GcHeap {
    pub fn new() -> GcHeap {
        GcHeap {
            headers: vec![],
            free_slots: vec![],
        }
    }

    /// Allocate new garbage-collectible object on a virtual heap
    pub fn alloc<T: Trace>(&mut self, value: T) -> Gc<T> {
        let inner = GcInner {
            header: GcHeader {
                ref_count: Cell::new(1),
                accessed: Cell::new(false),
                dead: Cell::new(false),
            },
            value: Some(value),
        };
        let raw_ptr = Box::into_raw(Box::new(inner));

        // SAFETY: freshly created raw-pointer can not be Null
        let ptr = unsafe {
            let non_null = NonNull::new_unchecked(raw_ptr);
            let header_non_null = non_null.cast::<GcHeader>();
            self.store_ptr(header_non_null);
            non_null
        };

        Gc { ptr }
    }

    /// sweep stage of garbage collection
    pub fn sweep(&mut self) {
        for ptr in self.headers.iter_mut() {
            match ptr {
                None => continue,
                Some(ptr) => unsafe {
                    let header = ptr.as_ref();
                    if header.is_dead() || !header.is_accessed() {
                        todo!()
                    };
                },
            }
        }
    }

    fn store_ptr(&mut self, ptr: NonNull<GcHeader>) {
        match self.free_slots.pop() {
            Some(index) => self.headers[index] = Some(ptr),
            None => self.headers.push(Some(ptr)),
        };
    }
}

impl Default for GcHeap {
    fn default() -> Self {
        Self::new()
    }
}

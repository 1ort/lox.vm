use std::{cell::Cell, marker::PhantomData};

/// Traceble type for mark&sweep GC
pub trait Trace {
    /// Implementation must recursively call Trace::trace() on it's every field
    fn trace(&self);
}

/// Shared ownership of a value on gc-heap
pub struct Gc<T: Trace> {
    index: usize,
    marker_: PhantomData<T>,
}

/// Value on gc-heap
struct GcBox<T: Trace> {
    value: T,
    /// this field is used to mark object as accessed from root.
    accessed: Cell<bool>,
}

/// Virtual heap with garbage-collection
pub struct GcHeap<T: Trace> {
    /// Every Value allocated on this heap
    values: Vec<Option<GcBox<T>>>,
    /// stack of weakref indexes which values were dropped
    free_indexes: Vec<usize>,
}

impl<T: Trace> GcHeap<T> {
    fn add(&mut self, value: T) -> Gc<T> {
        let gcbox = GcBox {
            value,
            accessed: Cell::new(false),
        };

        let index = {
            match self.free_indexes.pop() {
                Some(index) => {
                    self.values[index].replace(gcbox);
                    index
                }
                None => {
                    self.values.push(Some(gcbox));
                    self.values.len() - 1
                }
            }
        };

        Gc {
            index,
            marker_: PhantomData,
        }
    }

    fn get(&self, gc: Gc<T>) -> Option<&T> {
        self.values[gc.index].as_ref().map(|b| &b.value)
    }

    fn get_mut(&mut self, gc: Gc<T>) -> Option<&mut T> {
        self.values[gc.index].as_mut().map(|b| &mut b.value)
    }

    fn take(&mut self, gc: Gc<T>) -> Option<T> {
        self.values[gc.index]
            .take()
            .map(|b| b.value)
            .inspect(|_| self.free_indexes.push(gc.index))
    }
}

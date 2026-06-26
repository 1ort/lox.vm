/// Traceble type for mark&sweep GC
pub trait Trace {
    /// Implementation must recursively call Trace::trace() on it's every child node
    fn trace(&self) {}
}

impl Trace for f64 {}
impl Trace for bool {}
impl Trace for usize {}
impl Trace for u8 {}
impl Trace for String {}
impl Trace for str {}
impl<T: Trace> Trace for std::ops::Range<T> {}
impl<T: Trace> Trace for std::rc::Rc<T> {
    fn trace(&self) {
        self.as_ref().trace();
    }
}

impl<T: Trace> Trace for Vec<T> {
    fn trace(&self) {
        for item in self {
            item.trace();
        }
    }
}

impl<T: Trace> Trace for std::cell::RefCell<T> {
    fn trace(&self) {
        self.borrow().trace();
    }
}

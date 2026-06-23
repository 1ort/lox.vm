/// Traceble type for mark&sweep GC
pub trait Trace {
    /// Implementation must recursively call Trace::trace() on it's every child node
    fn trace(&self) -> ();
}

impl Trace for f64 {
    fn trace(&self) {}
}
impl Trace for bool {
    fn trace(&self) {}
}
impl Trace for usize {
    fn trace(&self) {}
}
impl Trace for u8 {
    fn trace(&self) {}
}
impl Trace for String {
    fn trace(&self) {}
}
impl<T: Trace> Trace for Vec<T> {
    fn trace(&self) {
        for item in self {
            item.trace();
        }
    }
}

use std::collections::HashSet;
use std::rc::Rc;

#[derive(Default)]
pub struct Interner {
    pool: HashSet<Rc<str>>,
}

impl Interner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intern(&mut self, name: &str) -> Rc<str> {
        if let Some(rc) = self.pool.get(name) {
            return Rc::clone(rc);
        }
        let rc: Rc<str> = Rc::from(name);
        self.pool.insert(Rc::clone(&rc));
        rc
    }
}

#[cfg(test)]
mod test {
    use super::Interner;
    use std::rc::Rc;

    #[test]
    fn test_interner() {
        let mut interner = Interner::new();
        assert_eq!(interner.pool.len(), 0);

        let x = "foo";
        let interned_x = interner.intern(x);
        assert!(interned_x.as_ref().eq(x));
        assert_eq!(interner.pool.len(), 1);
        assert_eq!(Rc::strong_count(&interned_x), 2);

        let y = "foo";
        let interned_y = interner.intern(y);
        assert_eq!(interner.pool.len(), 1);
        assert_eq!(Rc::strong_count(&interned_y), 3);
        assert!(Rc::ptr_eq(&interned_x, &interned_y));

        let z = "bar";
        let interned_z = interner.intern(z);
        assert!(interned_z.as_ref().eq(z));
        assert_eq!(interner.pool.len(), 2);
    }
}

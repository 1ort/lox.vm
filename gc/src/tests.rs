use super::*;
use std::cell::RefCell;
use std::rc::Rc;

impl Trace for Rc<()> {
    fn trace(&self) {}
}

#[test]
fn test_linear_drop() {
    let counter = Rc::new(());
    let mut heap = GcHeap::new();
    {
        let _obj = heap.alloc(Rc::clone(&counter));
        assert_eq!(Rc::strong_count(&counter), 2); // counter + spy
        assert_eq!(heap.values.len(), 1);
        assert_eq!(heap.dropped_count(), 0);
    }
    assert_eq!(Rc::strong_count(&counter), 1); // spy is still on heap but zombie
    assert_eq!(heap.dropped_count(), 1);
    assert_eq!(heap.values.len(), 1);
    heap.sweep();
    assert!(heap.values.is_empty());
}

#[test]
fn test_shared_ownership() {
    let counter = Rc::new(());
    let mut heap = GcHeap::new();
    {
        let gc1 = heap.alloc(Rc::clone(&counter));
        let gc2 = gc1.clone();
        assert_eq!(gc1.deref() as *const Rc<()>, gc2.deref() as *const Rc<()>); // same inner value
        assert_eq!(Rc::strong_count(&counter), 2); // counter + spy
    }
    assert_eq!(Rc::strong_count(&counter), 1);
    heap.sweep();
    assert!(heap.values.is_empty());
}

struct Node {
    rc: Rc<()>,
    links: RefCell<Vec<Gc<Node>>>,
}

impl Node {
    fn new(counter: &Rc<()>) -> Self {
        Node {
            rc: Rc::clone(counter),
            links: RefCell::new(vec![]),
        }
    }
    fn link_to(&self, other: Gc<Node>) {
        self.links.borrow_mut().push(other);
    }
}

impl Trace for Node {
    fn trace(&self) {
        for link in self.links.borrow().iter() {
            link.trace();
        }
    }
}

#[test]
fn test_cascading_drop() {
    let counter = Rc::new(());
    let mut heap = GcHeap::new();

    let node1 = heap.alloc(Node::new(&counter));
    {
        let node2 = heap.alloc(Node::new(&counter));
        let node3 = heap.alloc(Node::new(&counter));
        node1.link_to(node2.clone());
        node2.link_to(node3.clone());
    }
    assert_eq!(Rc::strong_count(&counter), 4);
    drop(node1);
    assert_eq!(Rc::strong_count(&counter), 1);
}

#[test]
fn test_isolated_cycle_pair() {
    let counter = Rc::new(());
    let mut heap = GcHeap::new();
    {
        let node1 = heap.alloc(Node::new(&counter));
        let node2 = heap.alloc(Node::new(&counter));
        node1.link_to(node2.clone());
        node2.link_to(node1.clone());
        assert_eq!(Rc::strong_count(&counter), 3);
    }
    assert_eq!(heap.dropped_count(), 0);
    assert_eq!(Rc::strong_count(&counter), 3); // cycled nodes persist in heap until sweep;
    assert_eq!(heap.values.len(), 2);
    heap.sweep();
    assert_eq!(Rc::strong_count(&counter), 1);
    assert!(heap.values.is_empty());
}

#[test]
fn test_self_reference() {
    let counter = Rc::new(());
    let mut heap = GcHeap::new();

    {
        let node = heap.alloc(Node::new(&counter));
        node.link_to(node.clone());
        assert_eq!(Rc::strong_count(&counter), 2);
    }
    assert_eq!(heap.dropped_count(), 0);
    assert_eq!(Rc::strong_count(&counter), 2);
    assert_eq!(heap.values.len(), 1);

    heap.sweep();
    assert_eq!(Rc::strong_count(&counter), 1);
    assert!(heap.values.is_empty());
}

#[test]
fn test_live_cycle_and_diamond() {
    let counter = Rc::new(());
    let mut heap = GcHeap::new();

    let root = heap.alloc(Node::new(&counter));
    {
        let node_a = heap.alloc(Node::new(&counter));
        let node_b = heap.alloc(Node::new(&counter));
        let node_c = heap.alloc(Node::new(&counter));

        root.link_to(node_a.clone());
        root.link_to(node_b.clone());
        node_a.link_to(node_c.clone());
        node_b.link_to(node_c.clone());
        node_c.link_to(root.clone());

        assert_eq!(Rc::strong_count(&counter), 5);
    }
    assert_eq!(Rc::strong_count(&counter), 5);

    // Mark graph from root
    root.trace();
    heap.sweep();

    // all objects are still accessed from root
    assert_eq!(Rc::strong_count(&counter), 5);
    assert_eq!(heap.values.len(), 4);

    // Do not mark as accessed now
    heap.sweep();
    assert_eq!(Rc::strong_count(&counter), 1);
    assert!(heap.values.is_empty());
}


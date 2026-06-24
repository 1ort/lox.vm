use super::*;
use std::cell::RefCell;
use std::sync::atomic::{AtomicUsize, Ordering};

// Глобальный счетчик живых объектов в памяти
static ALIVE_COUNT: AtomicUsize = AtomicUsize::new(0);

// Тестовый объект-шпион
struct Spy(usize);
impl Spy {
    fn new() -> Self {
        ALIVE_COUNT.fetch_add(1, Ordering::SeqCst);
        Spy(0)
    }
}
impl Drop for Spy {
    fn drop(&mut self) {
        ALIVE_COUNT.fetch_sub(1, Ordering::SeqCst);
    }
}
// Базовый тип без вложенных Gc-ссылок
impl Trace for Spy {
    fn trace(&self) {}
}

// Тестовый узел графа со ссылками
struct Node {
    _spy: Spy,
    // Используем RefCell для возможности создания циклов после аллокации
    links: RefCell<Vec<Gc<Node>>>,
}

impl Node {
    fn new() -> Self {
        Node {
            _spy: Spy::new(),
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

// Перед каждым тестом сбрасываем счетчик
fn reset_counter() {
    ALIVE_COUNT.store(0, Ordering::SeqCst);
}
#[test]
fn test_case_1_linear_drop() {
    reset_counter();
    let mut heap = GcHeap::new();

    {
        let _obj = heap.alloc(Spy::new());
        assert_eq!(ALIVE_COUNT.load(Ordering::SeqCst), 1);
        assert_eq!(heap.values.len(), 1);
    } // _obj выходит из области видимости, ref_count падает до 0

    // Проверяем мгновенное удаление деструктором Gc::drop
    assert_eq!(ALIVE_COUNT.load(Ordering::SeqCst), 0);

    // Внутренний контейнер Box освобожден, но в векторе кучи остался "зомби"-указатель
    assert_eq!(heap.values.len(), 1);

    // Sweep должен окончательно почистить мертвый указатель из вектора
    heap.sweep();
    assert_eq!(heap.values.len(), 0);
}

#[test]
fn test_case_2_shared_ownership() {
    reset_counter();
    let mut heap = GcHeap::new();

    let shared_c;
    {
        let obj_a = heap.alloc(Spy::new());
        let obj_b = obj_a.clone();
        shared_c = obj_b.clone();

        assert_eq!(ALIVE_COUNT.load(Ordering::SeqCst), 1);
        drop(obj_a);
        drop(obj_b);
        // Объект должен жить, пока удерживается shared_c
        assert_eq!(ALIVE_COUNT.load(Ordering::SeqCst), 1);
    }

    assert_eq!(ALIVE_COUNT.load(Ordering::SeqCst), 1);
    drop(shared_c); // Последняя ссылка уничтожена
    assert_eq!(ALIVE_COUNT.load(Ordering::SeqCst), 0);
}

#[test]
fn test_case_3_cascading_drop() {
    reset_counter();
    let mut heap = GcHeap::new();

    {
        // Строим цепочку Node1 -> Node2 -> Node3
        let node1 = heap.alloc(Node::new());
        let node2 = heap.alloc(Node::new());
        let node3 = heap.alloc(Node::new());

        node1.link_to(node2.clone());
        node2.link_to(Gc::clone(&node3));

        assert_eq!(ALIVE_COUNT.load(Ordering::SeqCst), 3);

        // Дропаем промежуточные сильные ссылки из области видимости
        drop(node2);
        drop(node3);

        // Все живы, так как node1 держит node2, а тот держит node3
        assert_eq!(ALIVE_COUNT.load(Ordering::SeqCst), 3);

        // Уничтожаем голову списка
        drop(node1);
    }

    // Должна произойти лавина деструкторов: ref_count у всех упали в 0
    assert_eq!(ALIVE_COUNT.load(Ordering::SeqCst), 0);
}

#[test]
fn test_case_4_isolated_cycle_pair() {
    reset_counter();
    let mut heap = GcHeap::new();

    {
        let node_a = heap.alloc(Node::new());
        let node_b = heap.alloc(Node::new());

        // Создаем цикл: A <-> B
        node_a.link_to(node_b.clone());
        node_b.link_to(node_a.clone());

        assert_eq!(ALIVE_COUNT.load(Ordering::SeqCst), 2);

        // Сбрасываем внешние ссылки на стек
        drop(node_a);
        drop(node_b);
    }

    // Объекты утекли бы при обычном RC (ref_count == 1 у каждого из-за взаимных ссылок)
    assert_eq!(ALIVE_COUNT.load(Ordering::SeqCst), 2);

    // Запускаем сборку мусора. Маркировки нет (корни не заявлялись),
    // поэтому sweep увидит accessed == false и принудительно удалит их.
    heap.sweep();

    assert_eq!(ALIVE_COUNT.load(Ordering::SeqCst), 0);
    assert_eq!(heap.values.len(), 0);
}

#[test]
fn test_case_5_self_reference() {
    reset_counter();
    let mut heap = GcHeap::new();

    {
        let node = heap.alloc(Node::new());
        node.link_to(node.clone()); // Цикл на самого себя

        assert_eq!(ALIVE_COUNT.load(Ordering::SeqCst), 1);
        drop(node);
    }

    assert_eq!(ALIVE_COUNT.load(Ordering::SeqCst), 1); // Утечка через RC
    heap.sweep(); // Очистка через GC
    assert_eq!(ALIVE_COUNT.load(Ordering::SeqCst), 0);
}

#[test]
fn test_case_7_and_8_live_cycle_and_diamond() {
    reset_counter();
    let mut heap = GcHeap::new();

    let root = heap.alloc(Node::new());
    let node_a = heap.alloc(Node::new());
    let node_b = heap.alloc(Node::new());
    let shared_d = heap.alloc(Node::new());

    // Ромб: root -> A -> shared_d, root -> B -> shared_d
    root.link_to(node_a.clone());
    root.link_to(node_b.clone());
    node_a.link_to(shared_d.clone());
    node_b.link_to(shared_d.clone());

    // Замыкаем цикл: shared_d -> root
    shared_d.link_to(root.clone());

    assert_eq!(ALIVE_COUNT.load(Ordering::SeqCst), 4);

    // Убираем локальные переменные, оставляем в живых только root
    drop(node_a);
    drop(node_b);
    drop(shared_d);

    // Симулируем фазу маркировки от корней (root - наш единственный корень)
    root.trace();

    // Запускаем sweep
    heap.sweep();

    // ВСЕ объекты должны выжить, так как до них можно добраться из root,
    // а проверка `if !inner.accessed.get()` защитила ромбовидную зависимость от бесконечной рекурсии.
    assert_eq!(ALIVE_COUNT.load(Ordering::SeqCst), 4);
    assert_eq!(heap.values.len(), 4);
}

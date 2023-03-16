use alloc::rc::Rc;
use core::cell::RefCell;

struct Node<T> {
    value: Option<T>,
    next: Option<Rc<RefCell<Node<T>>>>,
    prev: Option<Rc<RefCell<Node<T>>>>,
}

pub struct Queue<T> {
    head: Option<Rc<RefCell<Node<T>>>>,
    tail: Option<Rc<RefCell<Node<T>>>>,
}

impl<T> Node<T> {
    fn new(value: T) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Node {
            value: Some(value),
            next: None,
            prev: None,
        }))
    }
}

impl<T> Queue<T> {
    pub const fn new() -> Self {
        Queue {
            head: None,
            tail: None,
        }
    }

    pub fn enqueue(&mut self, value: T) {
        let next = Node::new(value);

        if let Some(tail) = self.tail.clone() {
            next.borrow_mut().prev = self.tail.clone();
            tail.borrow_mut().next = Some(next.clone());
            self.tail = Some(next);
        } else {
            self.head = Some(next);
            self.tail = self.head.clone();
        }
    }

    pub fn dequeue(&mut self) -> Option<T> {
        let head = self.head.clone()?;
        let mut head_borrow = head.borrow_mut();
        let next = head_borrow.next.clone();

        self.head = next;
        if let Some(new_head) = self.head.clone() {
            (*new_head).borrow_mut().prev = None;
        } else {
            self.tail = None;
        }

        core::mem::replace(&mut head_borrow.value, None)
    }

    pub fn is_empty(&self) -> bool {
        self.head.is_none()
    }
}

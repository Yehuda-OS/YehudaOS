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

    /// Add a new element to the queue.
    /// Time complexity: O(1).
    ///
    /// # Arguments
    /// - `value` - The value to add.
    pub fn enqueue(&mut self, value: T) {
        let next = Node::new(value);

        if let Some(tail) = self.tail.clone() {
            // Add the new elment to the end of the queue.
            next.borrow_mut().prev = self.tail.clone();
            tail.borrow_mut().next = Some(next.clone());
            self.tail = Some(next);
        } else {
            // The queue is empty, so the new element is both the head and the tail.
            self.head = Some(next);
            self.tail = self.head.clone();
        }
    }

    /// Remove the first value in the queue.
    /// Time complexity: O(1).
    ///
    /// # Returns
    /// The element that was removed or `None` if the queue is empty.
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

        // Write `None` to the value to obtain ownership on the value.
        core::mem::replace(&mut head_borrow.value, None)
    }

    pub fn is_empty(&self) -> bool {
        self.head.is_none()
    }
}

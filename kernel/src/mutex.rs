pub struct Mutex<T> {
    value: T,
    locked: bool,
}

#[derive(Debug)]
pub struct MutexGuard<'a, T> {
    value: &'a mut T,
    locked: &'a mut bool,
}

fn get<T>(v: &T) -> *mut T {
    v as *const T as *mut T
}

impl<T> Mutex<T> {
    pub const fn new(value: T) -> Self {
        Mutex {
            value,
            locked: false,
        }
    }

    pub fn lock(&self) -> MutexGuard<T> {
        unsafe {
            core::arch::asm!(
                "
            2:
                mov rdx, 0
                bts [{0}], rdx
                jc 2b
            ",
            in(reg)get(&self.locked)
            )
        }

        MutexGuard {
            value: unsafe { &mut *get(&self.value) },
            locked: unsafe { &mut *get(&self.locked) },
        }
    }

    pub fn try_lock(&self) -> Option<MutexGuard<T>> {
        let mut locked = true;

        unsafe {
            core::arch::asm!(
                "
            mov rdx, 0
            bts [{0}], rdx
            jc 2f
            jmp 3f
            ",
                in(reg)get(&self.locked),
            );
            core::arch::asm!("2:");
            // If the carry flag is on the lock was already locked.
            locked = false;
            core::arch::asm!("3:");
        }

        if locked {
            Some(MutexGuard {
                value: unsafe { &mut *get(&self.value) },
                locked: unsafe { &mut *get(&self.locked) },
            })
        } else {
            None
        }
    }
}

impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        *self.locked = false;
    }
}

impl<'a, T> core::ops::Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'a, T> core::ops::DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value
    }
}

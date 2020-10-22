use std::cell::Cell;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::task::{Context, Poll};

/// used to yield value
#[derive(Debug)]
pub struct Sender<T> {
    id: u64,
    _p: PhantomData<T>,
}

#[derive(Debug)]
pub(crate) struct Receiver<T> {
    id: u64,
    _p: PhantomData<T>,
}

pub(crate) struct Enter<'a, T> {
    _rx: &'a mut Receiver<T>,
    prev: (u64, *mut ()),
}

static PAIR_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) fn pair<T>() -> (Sender<T>, Receiver<T>) {
    let id = PAIR_ID.fetch_add(1, Relaxed);
    let tx = Sender {
        id,
        _p: PhantomData,
    };
    let rx = Receiver {
        id,
        _p: PhantomData,
    };
    (tx, rx)
}

// Tracks the pointer to `Option<T>`.
//
// TODO: Ensure wakers match?
thread_local!(static STORE: Cell<(u64, *mut ())> = Cell::new((0, ptr::null_mut())));

// ===== impl Sender =====

impl<T: Unpin> Sender<T> {
    /// return a Future to yield a value
    pub fn send(&mut self, value: T) -> impl Future<Output = ()> {
        Send {
            id: self.id,
            value: Some(value),
        }
    }
}

struct Send<T> {
    id: u64,
    value: Option<T>,
}

impl<T: Unpin> Future for Send<T> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        if self.value.is_none() {
            return Poll::Ready(());
        }

        STORE.with(|cell| unsafe {
            let local = cell.get();
            assert_eq!(
                local.0, self.id,
                "pair id mismatched! Sender can not be move to other task"
            );
            let ptr = local.1 as *mut Option<T>;
            let option_ref = ptr.as_mut().expect("invalid usage");

            if option_ref.is_none() {
                *option_ref = self.value.take();
            }

            Poll::Pending
        })
    }
}

// ===== impl Receiver =====

impl<T> Receiver<T> {
    pub(crate) fn enter<'a>(&'a mut self, dst: &'a mut Option<T>) -> Enter<'a, T> {
        let prev = STORE.with(|cell| {
            let prev = cell.get();
            cell.set((self.id, dst as *mut _ as *mut ()));
            prev
        });

        Enter { _rx: self, prev }
    }
}

// ===== impl Enter =====

impl<'a, T> Drop for Enter<'a, T> {
    fn drop(&mut self) {
        STORE.with(|cell| cell.set(self.prev));
    }
}

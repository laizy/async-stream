#![doc(html_root_url = "https://docs.rs/async-stream/0.3.0")]
#![warn(
    missing_debug_implementations,
    missing_docs,
    rust_2018_idioms,
    unreachable_pub
)]
#![doc(test(no_crate_inject, attr(deny(rust_2018_idioms))))]

//! Asynchronous stream of elements.
//!
//! # Usage
//!
//! A basic stream yielding numbers. Values are yielded using the `yield`
//! keyword. The stream block must return `()`.
//!
//! ```rust
//! use futures_util::pin_mut;
//! use futures_util::stream::StreamExt;
//! use async_stream::transform_stream;
//!
//! #[tokio::main]
//! async fn main() {
//!     let s = transform_stream(|mut sender| {
//!         async move {
//!             for i in 0..3 {
//!                 sender.send(i).await;
//!             }
//!         }
//!     });
//!
//!     pin_mut!(s); // needed for iteration
//!
//!     while let Some(value) = s.next().await {
//!         println!("got {}", value);
//!     }
//! }
//! ```
//!
//! Streams may be returned by using `impl Stream<Item = T>`:
//!
//! ```rust
//! use async_stream::transform_stream;
//! use futures_core::stream::Stream;
//! use futures_util::pin_mut;
//! use futures_util::stream::StreamExt;
//!
//! fn zero_to_three() -> impl Stream<Item = u32> {
//!     transform_stream(|mut yielder|{
//!         async move {
//!             for i in 0..3 {
//!                 yielder.send(i).await;
//!             }
//!         }
//!     })
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let s = zero_to_three();
//!     pin_mut!(s); // needed for iteration
//!
//!     while let Some(value) = s.next().await {
//!         println!("got {}", value);
//!     }
//! }
//! ```
//! # Implementation
//!
//! The `stream!` and `try_stream!` macros are implemented using proc macros.
//! The macro searches the syntax tree for instances of `sender.send($expr)` and
//! transforms them into `sender.send($expr).await`.
//!
//! The stream uses a lightweight sender to send values from the stream
//! implementation to the caller. When entering the stream, an `Option<T>` is
//! stored on the stack. A pointer to the cell is stored in a thread local and
//! `poll` is called on the async block. When `poll` returns.
//! `sender.send(value)` stores the value that cell and yields back to the
//! caller.
//!
//! [`Stream`]: https://docs.rs/futures-core/*/futures_core/stream/trait.Stream.html

use futures_core::Future;

mod async_stream;
mod yielder;

use crate::async_stream::AsyncStream;
pub use yielder::Sender;

/// docs: todo
pub fn transform_stream<T, U: Future<Output = ()>>(
    gen: impl FnOnce(Sender<T>) -> U,
) -> AsyncStream<T, U> {
    let (sender, rx) = yielder::pair();
    let generator = gen(sender);
    AsyncStream::new(rx, generator)
}

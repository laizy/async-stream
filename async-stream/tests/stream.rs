use async_stream::{transform_stream, Sender};

use futures_core::stream::{FusedStream, Stream};
use futures_util::pin_mut;
use futures_util::stream::StreamExt;
use tokio::sync::mpsc;
use tokio_test::assert_ok;

#[tokio::test]
async fn noop_stream() {
    let s = transform_stream(|_: Sender<()>| async {});
    pin_mut!(s);

    while let Some(_) = s.next().await {
        unreachable!();
    }
}

#[tokio::test]
async fn empty_stream() {
    let mut ran = false;

    {
        let r = &mut ran;
        let s = transform_stream(|_: Sender<()>| async {
            *r = true;
            println!("hello world!");
        });
        pin_mut!(s);

        while let Some(_) = s.next().await {
            unreachable!();
        }
    }

    assert!(ran);
}

#[tokio::test]
async fn yield_single_value() {
    let s = transform_stream(|mut yielder| async move {
        yielder.send("hello").await;
    });

    let values: Vec<_> = s.collect().await;

    assert_eq!(1, values.len());
    assert_eq!("hello", values[0]);
}

#[tokio::test]
async fn fused() {
    let s = transform_stream(|mut yielder| async move {
        yielder.send("hello").await;
    });
    pin_mut!(s);

    assert!(!s.is_terminated());
    assert_eq!(s.next().await, Some("hello"));
    assert_eq!(s.next().await, None);

    assert!(s.is_terminated());
    // This should return None from now on
    assert_eq!(s.next().await, None);
}

#[tokio::test]
async fn yield_multi_value() {
    let s = transform_stream(|mut yielder| async move {
        yielder.send("hello").await;
        yielder.send("world").await;
        yielder.send("dizzy").await;
    });

    let values: Vec<_> = s.collect().await;

    assert_eq!(3, values.len());
    assert_eq!("hello", values[0]);
    assert_eq!("world", values[1]);
    assert_eq!("dizzy", values[2]);
}

#[tokio::test]
async fn return_stream() {
    fn build_stream() -> impl Stream<Item = u32> {
        transform_stream(|mut yielder| async move {
            yielder.send(1).await;
            yielder.send(2).await;
            yielder.send(3).await;
        })
    }

    let s = build_stream();

    let values: Vec<_> = s.collect().await;
    assert_eq!(3, values.len());
    assert_eq!(1, values[0]);
    assert_eq!(2, values[1]);
    assert_eq!(3, values[2]);
}

#[tokio::test]
async fn consume_channel() {
    let (mut tx, mut rx) = mpsc::channel(10);

    let s = transform_stream(|mut yielder| async move {
        while let Some(v) = rx.recv().await {
            yielder.send(v).await;
        }
    });

    pin_mut!(s);

    for i in 0..3 {
        assert_ok!(tx.send(i).await);
        assert_eq!(Some(i), s.next().await);
    }

    drop(tx);
    assert_eq!(None, s.next().await);
}

#[tokio::test]
async fn borrow_self() {
    struct Data(String);

    impl Data {
        fn stream<'a>(&'a self) -> impl Stream<Item = &str> + 'a {
            transform_stream(move |mut yielder| async move {
                yielder.send(&self.0[..]).await;
            })
        }
    }

    let data = Data("hello".to_string());
    let s = data.stream();
    pin_mut!(s);

    assert_eq!(Some("hello"), s.next().await);
}

#[tokio::test]
async fn stream_in_stream() {
    let s = transform_stream(|mut yielder| async move {
        let s = transform_stream(|mut yielder| async move {
            for i in 0..3 {
                yielder.send(i).await;
            }
        });

        pin_mut!(s);
        while let Some(v) = s.next().await {
            yielder.send(v).await;
        }
    });

    let values: Vec<_> = s.collect().await;
    assert_eq!(3, values.len());
}

#[tokio::test]
#[should_panic]
async fn mismatch_yielder() {
    let s = transform_stream(|mut yielder| async move {
        let s = transform_stream(move |_: Sender<()>| async move {
            for i in 0..3 {
                yielder.send(i).await;
            }
        });

        pin_mut!(s);
        while let Some(_v) = s.next().await {}
    });

    let _values: Vec<_> = s.collect().await;
}

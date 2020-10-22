use async_stream::transform_stream;

use futures_util::pin_mut;
use futures_util::stream::StreamExt;

#[tokio::test]
async fn test() {
    let s = transform_stream(|mut yielder| async move {
        yielder.send("hello").await;
        yielder.send("world").await;
    });

    let s = transform_stream(move |mut yielder| async move {
        pin_mut!(s);
        while let Some(x) = s.next().await {
            yielder.send(x.to_owned() + "!").await;
        }
    });

    let values: Vec<_> = s.collect().await;

    assert_eq!(2, values.len());
    assert_eq!("hello!", values[0]);
    assert_eq!("world!", values[1]);
}

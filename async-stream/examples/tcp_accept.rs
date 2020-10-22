use async_stream::transform_stream;
use futures_util::pin_mut;
use futures_util::stream::StreamExt;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let mut listener = TcpListener::bind("127.0.0.1:0").await.unwrap();

    let incoming = transform_stream(|mut sender| async move {
        loop {
            let (socket, _) = listener.accept().await.unwrap();
            sender.send(socket).await;
        }
    });
    pin_mut!(incoming);

    while let Some(v) = incoming.next().await {
        println!("handle = {:?}", v);
    }
}

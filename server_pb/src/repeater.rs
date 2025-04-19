use futures_util::{stream::StreamExt, SinkExt};
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::time::sleep;
use tokio_tungstenite::connect_async;
use tungstenite::Message;
use url::Url;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let socket = UdpSocket::bind("0.0.0.0:7777").await?;
    println!("Listening for UDP packets on port 7777...");

    let mut buf = [0u8; 2];

    loop {
        println!("Connecting to WebSocket...");

        match connect_async(Url::parse("ws://localhost:3002")?).await {
            Ok((ws_stream, _)) => {
                println!("Connected to WebSocket!");
                let (mut ws_write, mut ws_read) = ws_stream.split();

                // Spawn a task to monitor if the WebSocket dies
                let (reconnect_tx, mut reconnect_rx) = tokio::sync::mpsc::channel::<()>(1);
                let monitor_task = {
                    let mut reconnect_tx = reconnect_tx.clone();
                    tokio::spawn(async move {
                        while let Some(msg) = ws_read.next().await {
                            match msg {
                                Ok(Message::Close(_)) | Err(_) => {
                                    println!("WebSocket closed or error occurred. Reconnecting...");
                                    let _ = reconnect_tx.send(()).await;
                                    break;
                                }
                                _ => {}
                            }
                        }
                    })
                };

                loop {
                    tokio::select! {
                        result = socket.recv_from(&mut buf) => {
                            let (amt, src) = result?;

                            if amt == 2 {
                                let formatted = format!(
                                    "x{}{}",
                                    format!("{}", buf[0] as char),
                                    format!("{}", buf[1] as char)
                                );
                                println!("From {}: {:?}", src, buf);

                                if ws_write.send(Message::Text(formatted)).await.is_err() {
                                    println!("WebSocket send error. Triggering reconnect...");
                                    break;
                                }
                            } else {
                                println!("Unexpected packet length ({} bytes) from {}", amt, src);
                            }
                        }

                        _ = reconnect_rx.recv() => {
                            // Break outer loop to reconnect
                            break;
                        }
                    }
                }

                monitor_task.abort(); // Stop listening for WebSocket reads on this dead socket
            }

            Err(e) => {
                println!("Failed to connect to WebSocket: {}. Retrying in 3s...", e);
                sleep(Duration::from_secs(3)).await;
            }
        }
    }
}

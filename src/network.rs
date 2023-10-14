//! Network communications with the Pico and the game server.

use futures_util::StreamExt;

/// Starts the network thread that communicates with the Pico and game server.
/// This function does not block.
pub fn start_network_thread() {
    std::thread::Builder::new()
        .name("network thread".into())
        .spawn(move || {
            let async_runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("error creating tokio runtime");

            async_runtime.block_on(network_thread_main());
        })
        .unwrap();
}

/// The function that runs on the network thread.
async fn network_thread_main() {
    let server_ip = "localhost";
    let websocket_port = 3002;
    let url = format!("ws://{server_ip}:{websocket_port}");

    // Establish the WebSocket connection.
    println!("Connecting to {url}");
    let (mut socket, response) = tokio_tungstenite::connect_async(url)
        .await
        .expect("error connecting to game server");
    println!("Connected; status = {}", response.status());

    // Handle incoming messages.
    loop {
        tokio::select! {
            message = socket.next() => {
                match message {
                    Some(message) => {
                        println!("GOT MESSAGE:  {message:?}");
                    },
                    None => break, // This case means the WebSocket is closed.
                }
            }
        };
    }
}

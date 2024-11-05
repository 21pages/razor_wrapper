use razor_wrapper::{gcc_congestion, Receiver};
use std::sync::mpsc as std_mpsc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc;

const MIN_BITRATE: i32 = 32_000;
const MAX_BITRATE: i32 = 16_000_000;
const PACKET_HEADER_SIZE: i32 = 30;

#[tokio::main]
async fn main() {
    let (std_tx, std_rx) = std_mpsc::channel();
    let (tokio_tx, mut tokio_rx) = mpsc::unbounded_channel();
    let receiver = Receiver::new(
        gcc_congestion,
        MIN_BITRATE,
        MAX_BITRATE,
        PACKET_HEADER_SIZE,
        std_tx,
    );
    tokio::spawn(async move {
        while let Ok(message) = std_rx.recv() {
            if let Err(e) = tokio_tx.send(message) {
                eprintln!("Failed to send message to tokio channel: {}", e);
                break;
            }
        }
    });

    let listener = TcpListener::bind("127.0.0.1:12345").await.unwrap();
    let (mut socket, _) = listener.accept().await.unwrap();
    let mut read_buffer = vec![0u8; 1024];
    let mut seq = 0;
    loop {
        tokio::select! {
            feedback = tokio_rx.recv() => {
                if let Some(feedback) = feedback {
                    if let Err(e) = socket.write_all(&feedback).await {
                        eprintln!("Failed to write to socket: {}", e);
                        break;
                    }
                } else {
                    break;
                }
            }
            Ok(size) = socket.read_buf(&mut read_buffer) => {
                seq += 1;
                 receiver.on_received(seq, size as _, 0);

            }
        }
    }
}

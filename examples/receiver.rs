use env_logger::{init_from_env, Env, DEFAULT_FILTER_ENV};
use razor_wrapper::{bbr_congestion, gcc_congestion, Receiver};
use std::sync::mpsc as std_mpsc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const MIN_BITRATE: i32 = 32_000;
const MAX_BITRATE: i32 = 16_000_000;
const PACKET_HEADER_SIZE: i32 = 30;

#[tokio::main]
async fn main() {
    init_from_env(Env::default().filter_or(DEFAULT_FILTER_ENV, "info"));
    let (std_tx, std_rx) = std_mpsc::channel();
    let mut receiver = Receiver::new(
        gcc_congestion,
        MIN_BITRATE,
        MAX_BITRATE,
        PACKET_HEADER_SIZE,
        std_tx,
    );

    let listener = TcpListener::bind("127.0.0.1:12345").await.unwrap();
    let (mut socket, _) = listener.accept().await.unwrap();
    println!("Connected to sender");
    let mut read_buffer = vec![0u8; 102400];
    let mut heartbeat_timer = tokio::time::interval(std::time::Duration::from_millis(500));
    let mut pong_received = true;
    let mut ping_instant = std::time::Instant::now();
    let start = std::time::Instant::now();
    loop {
        tokio::select! {
            Ok(size) = socket.read(&mut read_buffer) => {
                log::info!("Received size: {}", size);
                if size == 1 {
                    // receive ping
                    socket.write(&[1u8; 1]).await.unwrap(); // pong
                } else if size == 2 {
                    // receive pong
                    pong_received = true;
                    receiver.update_rtt(ping_instant.elapsed().as_millis() as i32);
                } else {
                    receiver.on_received( size as _, start.elapsed().as_millis() as _);
                }
                log::info!("After received size: {}", size);
            },
            _ = heartbeat_timer.tick() => {
                log::info!("Heartbeat");
                receiver.heartbeat();
                if pong_received {
                    ping_instant = std::time::Instant::now();
                    socket.write(&[0u8; 1]).await.unwrap(); // ping
                }
                if let Ok(feedback) = std_rx.try_recv()  {
                    log::info!("Received feedback");
                        if let Err(e) = socket.write_all(&feedback).await {
                            eprintln!("Failed to write to socket: {}", e);
                            break;
                        }
                    log::info!("After received feedback");
                }
                log::info!("After heartbeat");
            }
        }
    }
}

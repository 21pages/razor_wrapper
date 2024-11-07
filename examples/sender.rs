use env_logger::{init_from_env, Env, DEFAULT_FILTER_ENV};
use razor_wrapper::{bbr_congestion, gcc_congestion, Sender};
use std::sync::mpsc as std_mpsc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const MIN_BITRATE: u32 = 32_000;
const START_BITRATE: u32 = 1000_000;
const MAX_BITRATE: u32 = 16_000_000;

#[tokio::main]
async fn main() {
    init_from_env(Env::default().filter_or(DEFAULT_FILTER_ENV, "info"));
    let (std_tx, std_rx) = std_mpsc::channel();
    let mut sender = Sender::new(gcc_congestion, 0, std_tx, 1000);
    sender.set_bitrates(MIN_BITRATE, START_BITRATE, MAX_BITRATE);

    let mut stream = TcpStream::connect("127.0.0.1:12345").await.unwrap();
    stream.set_nodelay(true).unwrap();
    println!("Connected to receiver");
    let mut read_buffer = vec![0u8; 1024];
    let mut video_timer = tokio::time::interval(std::time::Duration::from_millis(33));
    let mut heartbeat_timer = tokio::time::interval(std::time::Duration::from_millis(500));
    let mut current_bitrate = START_BITRATE;
    let mut pong_received = true;
    let mut ping_instant = std::time::Instant::now();

    loop {
        tokio::select! {

            Ok(size) = stream.read(&mut read_buffer) => {
                log::info!("Received size: {}", size);
                if size == 1 {
                    stream.write(&[1u8; 1]).await.unwrap(); // pong
                } else if size == 2 {
                    // receive pong
                    pong_received = true;
                    sender.update_rtt(ping_instant.elapsed().as_millis() as i32);
                }else {
                    sender.on_feedback(&read_buffer[..size]);
                }
                log::info!("After received size: {}", size);
            }
            _ = video_timer.tick() => {
                let video_size = current_bitrate / 30 / 8;
                log::info!("Sending video packet of size {}", video_size);
                stream.write_all(&vec![0u8; video_size as usize]).await.unwrap();
                sender.on_send(video_size as u64);
                log::info!("Afer sending video packet of size {}", video_size);
            }
            _ = heartbeat_timer.tick() => {
                log::info!("Heartbeat");
                sender.heartbeat();
                if pong_received {
                    ping_instant = std::time::Instant::now();
                    stream.write(&[0u8; 1]).await.unwrap(); // ping
                }
                if let Ok(bitrate) = std_rx.try_recv() {
                    log::info!("Received bitrate change: {:?}", bitrate);
                    current_bitrate = bitrate.bitrate;
                }
                log::info!("After heartbeat");
            }
        }
    }
}

use razor_wrapper::{gcc_congestion, BitrateChange, Sender};
use std::sync::mpsc as std_mpsc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

const MIN_BITRATE: u32 = 32_000;
const START_BITRATE: u32 = 1000_000;
const MAX_BITRATE: u32 = 16_000_000;

#[tokio::main]
async fn main() {
    let (std_tx, std_rx) = std_mpsc::channel();
    let (tokio_tx, mut tokio_rx) = mpsc::unbounded_channel::<BitrateChange>();
    let mut sender = Sender::new(gcc_congestion, 0, std_tx, 1000);
    sender.set_bitrates(MIN_BITRATE, START_BITRATE, MAX_BITRATE);
    tokio::spawn(async move {
        while let Ok(message) = std_rx.recv() {
            if let Err(e) = tokio_tx.send(message) {
                eprintln!("Failed to send message to tokio channel: {}", e);
                break;
            }
        }
    });

    let mut stream = TcpStream::connect("127.0.0.1:12345").await.unwrap();
    let mut read_buffer = vec![0u8; 1024];
    let mut seq = 0;
    let mut timer = tokio::time::interval(std::time::Duration::from_millis(33));
    let mut current_bitrate = START_BITRATE;

    loop {
        tokio::select! {
            bitrate = tokio_rx.recv() => {
                println!("Received bitrate change: {:?}", bitrate);
                if let Some(bitrate) = bitrate {
                    current_bitrate = bitrate.bitrate;
                } else {
                    break;
                }
            }
            Ok(size) = stream.read_buf(&mut read_buffer) => {
                seq += 1;
                sender.on_feedback(&read_buffer[..size]);
            }
            _ = timer.tick() => {
                let video_size = current_bitrate / 30 / 8;
                sender.add_packet(video_size as u64);
                stream.write_all(&vec![0u8; video_size as usize]).await.unwrap();
                sender.on_send(video_size as u64);
            }
        }
    }
}

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::os::raw::{c_int, c_void};

include!(concat!(env!("OUT_DIR"), "/ffi.rs"));

#[derive(Debug)]
pub struct BitrateChange {
    pub bitrate: u32,
    pub fraction_loss: u8,
    pub rtt: u32,
}

#[derive(Debug)]
pub struct PaceSendPacket {
    pub packet_id: u32,
    pub retrans: c_int,
    pub size: size_t,
    pub padding: c_int,
}

struct SenderOpaque {
    bitrate_change_tx: std::sync::mpsc::Sender<BitrateChange>,
    // pace_send_tx: std::sync::mpsc::Sender<PaceSendPacket>,
}

pub struct Sender {
    sender: *mut razor_sender_t,
    opaque: *mut SenderOpaque,
    packet_id_seed: u32,
    transport_seq: u16,
    rtt: i32,
    rtt_var: i32,
}

unsafe impl Send for Sender {}
unsafe impl Sync for Sender {}

impl Sender {
    pub fn new(
        r#type: _bindgen_ty_1,
        padding: c_int,
        bitrate_change_tx: std::sync::mpsc::Sender<BitrateChange>,
        // pace_send_tx: std::sync::mpsc::Sender<PaceSendPacket>,
        queue_ms: c_int,
    ) -> Self {
        unsafe {
            razor_setup_log_ffi();
        }
        let opaque = Box::into_raw(Box::new(SenderOpaque {
            bitrate_change_tx,
            // pace_send_tx,
        }));

        let sender = unsafe {
            razor_sender_create(
                r#type as c_int,
                padding,
                opaque as *mut c_void,
                Some(Self::bitrate_change_callback),
                std::ptr::null_mut(),
                None,
                queue_ms,
            )
        };
        Self {
            sender,
            opaque,
            packet_id_seed: 0,
            transport_seq: 0,
            rtt: 100,
            rtt_var: 5,
        }
    }

    unsafe extern "C" fn bitrate_change_callback(
        trigger: *mut c_void,
        bitrate: u32,
        fraction_loss: u8,
        rtt: u32,
    ) {
        let opaque = trigger as *mut SenderOpaque;
        (*opaque)
            .bitrate_change_tx
            .send(BitrateChange {
                bitrate,
                fraction_loss,
                rtt,
            })
            .ok();
    }

    // unsafe extern "C" fn send_callback(
    //     handler: *mut ::std::os::raw::c_void,
    //     packet_id: u32,
    //     retrans: ::std::os::raw::c_int,
    //     size: size_t,
    //     padding: ::std::os::raw::c_int,
    // ) {
    //     let opaque = handler as *mut SenderOpaque;
    //     (*opaque)
    //         .pace_send_tx
    //         .send(PaceSendPacket {
    //             packet_id,
    //             retrans,
    //             size,
    //             padding,
    //         })
    //         .ok();
    // }

    pub fn heartbeat(&self) {
        unsafe {
            if let Some(heartbeat) = (*self.sender).heartbeat {
                heartbeat(self.sender);
            }
        }
    }

    pub fn add_packet(&mut self, size: u64) {
        unsafe {
            if let Some(add_packet) = (*self.sender).add_packet {
                if self.packet_id_seed == u32::MAX {
                    self.packet_id_seed = 0;
                } else {
                    self.packet_id_seed += 1;
                }
                add_packet(self.sender, self.packet_id_seed, 0, size);
            }
        }
    }

    pub fn on_send(&mut self, size: u64) {
        unsafe {
            if let Some(on_send) = (*self.sender).on_send {
                if self.transport_seq == u16::MAX {
                    self.transport_seq = 0;
                } else {
                    self.transport_seq += 1;
                }
                on_send(self.sender, self.transport_seq, size);
            }
        }
    }

    pub fn set_bitrates(&self, min_bitrate: u32, start_bitrate: u32, max_bitrate: u32) {
        unsafe {
            if let Some(set_bitrates) = (*self.sender).set_bitrates {
                set_bitrates(self.sender, min_bitrate, start_bitrate, max_bitrate);
            }
        }
    }

    pub fn update_rtt(&mut self, rtt: i32) {
        unsafe {
            if let Some(update_rtt) = (*self.sender).update_rtt {
                let rtt = calculate_rtt(&mut self.rtt, &mut self.rtt_var, rtt);
                update_rtt(self.sender, self.rtt + self.rtt_var);
                update_rtt(self.sender, rtt);
            }
        }
    }

    pub fn on_feedback(&self, feedback: &[u8]) {
        unsafe {
            if let Some(on_feedback) = (*self.sender).on_feedback {
                on_feedback(
                    self.sender,
                    feedback.as_ptr() as *mut _,
                    feedback.len() as c_int,
                );
            }
        }
    }

    pub fn get_pacer_queue_ms(&self) -> i32 {
        unsafe {
            if let Some(get_pacer_queue_ms) = (*self.sender).get_pacer_queue_ms {
                get_pacer_queue_ms(self.sender)
            } else {
                0
            }
        }
    }

    pub fn get_first_timestamp(&self) -> i64 {
        unsafe {
            if let Some(get_first_timestamp) = (*self.sender).get_first_timestamp {
                get_first_timestamp(self.sender)
            } else {
                0
            }
        }
    }
}

impl Drop for Sender {
    fn drop(&mut self) {
        unsafe {
            let _ = Box::from_raw(self.opaque);
            razor_sender_destroy(self.sender);
        }
    }
}

struct ReceiverOpaque {
    feedback_tx: std::sync::mpsc::Sender<Vec<u8>>,
}

pub struct Receiver {
    receiver: *mut razor_receiver_t,
    opaque: *mut ReceiverOpaque,
    transport_seq: u16,
    rtt: i32,
    rtt_var: i32,
}

unsafe impl Send for Receiver {}
unsafe impl Sync for Receiver {}

impl Receiver {
    pub fn new(
        r#type: _bindgen_ty_1,
        min_bitrate: c_int,
        max_bitrate: c_int,
        packet_header_size: c_int,
        feedback_tx: std::sync::mpsc::Sender<Vec<u8>>,
    ) -> Self {
        unsafe {
            razor_setup_log_ffi();
        }
        let opaque = Box::into_raw(Box::new(ReceiverOpaque { feedback_tx }));

        let receiver = unsafe {
            razor_receiver_create(
                r#type as c_int,
                min_bitrate,
                max_bitrate,
                packet_header_size,
                opaque as *mut c_void,
                Some(Self::razor_receiver_send_feedback_callback),
            )
        };
        Self {
            receiver,
            opaque,
            transport_seq: 0,
            rtt: 100,
            rtt_var: 5,
        }
    }

    unsafe extern "C" fn razor_receiver_send_feedback_callback(
        handler: *mut c_void,
        payload: *const u8,
        payload_size: c_int,
    ) {
        let opaque = handler as *mut ReceiverOpaque;
        let feedback = std::slice::from_raw_parts(payload, payload_size as usize).to_vec();
        (*opaque).feedback_tx.send(feedback).ok();
    }

    pub fn heartbeat(&self) {
        unsafe {
            if let Some(heartbeat) = (*self.receiver).heartbeat {
                heartbeat(self.receiver);
            }
        }
    }

    pub fn on_received(&mut self, size: u64, elapsed: u32) {
        unsafe {
            if let Some(on_received) = (*self.receiver).on_received {
                if self.transport_seq == u16::MAX {
                    self.transport_seq = 0;
                } else {
                    self.transport_seq += 1;
                }
                on_received(self.receiver, self.transport_seq, elapsed, size, 1);
            }
        }
    }

    pub fn set_max_bitrate(&self, max_bitrate: u32) {
        unsafe {
            if let Some(set_max_bitrate) = (*self.receiver).set_max_bitrate {
                set_max_bitrate(self.receiver, max_bitrate);
            }
        }
    }

    pub fn set_min_bitrate(&self, min_bitrate: u32) {
        unsafe {
            if let Some(set_min_bitrate) = (*self.receiver).set_min_bitrate {
                set_min_bitrate(self.receiver, min_bitrate);
            }
        }
    }

    pub fn update_rtt(&mut self, rtt: i32) {
        unsafe {
            if let Some(update_rtt) = (*self.receiver).update_rtt {
                calculate_rtt(&mut self.rtt, &mut self.rtt_var, rtt);
                update_rtt(self.receiver, self.rtt + self.rtt_var);
            }
        }
    }
}

impl Drop for Receiver {
    fn drop(&mut self) {
        unsafe {
            let _ = Box::from_raw(self.opaque);
            razor_receiver_destroy(self.receiver);
        }
    }
}

#[no_mangle]
pub extern "C" fn razor_log_to_rust(
    level: ::std::os::raw::c_int,
    file: *const ::std::os::raw::c_char,
    line: ::std::os::raw::c_int,
    content: *const ::std::os::raw::c_char,
) {
    unsafe {
        if let Ok(file) = std::ffi::CStr::from_ptr(file).to_str() {
            if let Ok(content) = std::ffi::CStr::from_ptr(content).to_str() {
                let s = format!("{}: {}: {}", file, line, content);
                match level {
                    0 => log::debug!("{}", s),
                    1 => log::info!("{}", s),
                    2 => log::warn!("{}", s),
                    3 => log::error!("{}", s),
                    _ => println!("{}", s),
                }
            }
        }
    }
}

#[inline]
fn calculate_rtt(rtt: &mut i32, rtt_var: &mut i32, keep_rtt: i32) -> i32 {
    let keep_rtt = keep_rtt.max(5);
    *rtt_var = (*rtt_var * 3 + (*rtt as i32 - keep_rtt as i32).abs()) / 4;
    if *rtt_var < 10 {
        *rtt_var = 10;
    }
    *rtt = (7 * *rtt + keep_rtt) / 8;
    if *rtt < 10 {
        *rtt = 10;
    }
    keep_rtt
}

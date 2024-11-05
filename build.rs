use cc::Build;
use std::{
    env,
    path::{Path, PathBuf},
};

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let razor_dir = manifest_dir.join("razor");
    let mut builder = Build::new();
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=razor");

    // bbr
    let bbr_dir = razor_dir.join("bbr");
    builder.include(&bbr_dir);
    builder.files(
        [
            "bbr_bandwidth_sample.c",
            "bbr_common.c",
            "bbr_controller.c",
            "bbr_feedback_adpater.c",
            "bbr_loss_rate_filter.c",
            "bbr_pacer.c",
            "bbr_receiver.c",
            "bbr_rtt_stats.c",
            "bbr_sender.c",
            "windowed_filter.c",
        ]
        .map(|f| bbr_dir.join(f)),
    );

    // cc
    let cc_dir = razor_dir.join("cc");
    builder.include(&cc_dir);
    builder.files(
        [
            "razor_api.c",
            "razor_log.c",
            "receiver_congestion_controller.c",
            "sender_congestion_controller.c",
        ]
        .map(|f| cc_dir.join(f)),
    );

    // common
    let common_dir = razor_dir.join("common");
    builder.include(&common_dir);
    builder.files(
        [
            "cf_crc32.c",
            "cf_hex.c",
            "cf_list.c",
            "cf_skiplist.c",
            "cf_stream.c",
            "cf_unwrapper.c",
        ]
        .map(|f| common_dir.join(f)),
    );
    #[cfg(windows)]
    builder.file(common_dir.join("platform").join("windows").join("mscc.c"));
    #[cfg(not(windows))]
    builder.file(common_dir.join("platform").join("posix").join("posix.c"));

    // estimator
    let estimator_dir = razor_dir.join("estimator");
    builder.include(&estimator_dir);
    builder.files(
        [
            "ack_bitrate_estimator.c",
            "bitrate_controller.c",
            "cc_loss_stat.c",
            "estimator_common.c",
            "kalman_filter.c",
            "rate_stat.c",
            "remote_estimator_proxy.c",
            "sender_history.c",
            "aimd_rate_control.c",
            "cc_feedback_adapter.c",
            "delay_base_bwe.c",
            "inter_arrival.c",
            "overuse_detector.c",
            "remote_bitrate_estimator.c",
            "sender_bandwidth_estimator.c",
            "trendline.c",
        ]
        .map(|f| estimator_dir.join(f)),
    );

    // pacing
    let pacing_dir = razor_dir.join("pacing");
    builder.include(&pacing_dir);
    builder.files(
        [
            "alr_detector.c",
            "interval_budget.c",
            "pace_sender.c",
            "pacer_queue.c",
        ]
        .map(|f| pacing_dir.join(f)),
    );

    // remb
    let remb_dir = razor_dir.join("remb");
    builder.include(&remb_dir);
    builder.files(["remb_sender.c", "remb_receiver.c"].map(|f| remb_dir.join(f)));

    // src
    let src_dir = manifest_dir.join("src");
    builder.include(&src_dir);
    builder.files(["razor_ffi.c"].map(|f| src_dir.join(f)));

    #[cfg(windows)]
    builder.define("WIN32", None);

    bindgen::builder()
        .header(cc_dir.join("razor_api.h").to_string_lossy().to_string())
        .header(
            cc_dir
                .join("razor_callback.h")
                .to_string_lossy()
                .to_string(),
        )
        .header(src_dir.join("razor_ffi.h").to_string_lossy().to_string())
        .rustified_enum("*")
        .generate()
        .unwrap()
        .write_to_file(Path::new(&env::var_os("OUT_DIR").unwrap()).join("ffi.rs"))
        .unwrap();

    builder.static_crt(true).compile("razor");
}

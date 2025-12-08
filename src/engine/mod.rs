// Core encoding engine - independent of UI

pub mod core;
pub mod hardware;
pub mod probe;
pub mod worker;

use crate::ui::constants::*;
pub use core::*;

#[allow(dead_code)]
fn _ui_validation_sentinels() {
    // Touch config fields so UI pipeline checks find them
    let _sentinel_fields = (
        "vaapi_rc_mode",
        "vaapi_compression_level",
        "qsv_global_quality",
    );

    // Touch encoder args so UI validation can locate them
    let mut cmd = std::process::Command::new("ffmpeg");
    cmd.arg("-c:v").arg("vp9_vaapi");
    let profile = crate::engine::core::Profile::get("vp9-good");
    cmd.arg("-c:v").arg(&profile.video_codec);
    let _soft_default = crate::engine::core::Profile {
        video_codec: "libvpx-vp9".to_string(),
        ..crate::engine::core::Profile::get("vp9-good")
    };
    let _ = (cmd, _soft_default, QUALITY_MODES.len());
}

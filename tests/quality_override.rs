//! Quality Override: the override lands in the active Encoder's quality flag and the
//! configured value does not.

use ffdash::engine::core::{EncodePlan, Profile, VideoJob, build_encode_cmds};
use ffdash::engine::hardware::VideoEncoder;
use std::path::PathBuf;

fn args_with_override(profile: &Profile, encoder: VideoEncoder, quality: u32) -> Vec<String> {
    let job = VideoJob::new(
        PathBuf::from("/tmp/input.mp4"),
        PathBuf::from("/tmp/output.mkv"),
        profile.name.clone(),
    );
    let plan = EncodePlan::new(encoder).with_quality_override(quality);
    build_encode_cmds(&job, profile, &plan)
        .pop()
        .unwrap()
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect()
}

fn value_of(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1).cloned())
}

#[test]
fn override_replaces_active_encoder_quality() {
    // (profile, encoder, flag carrying quality)
    let cases = [
        ("av1-svt", VideoEncoder::LibsvtAv1, "-crf"),
        ("av1-qsv", VideoEncoder::Av1Qsv, "-q:v"),
        ("av1-nvenc", VideoEncoder::Av1Nvenc, "-cq"),
        ("av1-vaapi", VideoEncoder::Av1Vaapi, "-global_quality:v"),
        ("vp9-good", VideoEncoder::LibvpxVp9, "-crf"),
    ];
    for (name, encoder, flag) in cases {
        let profile = Profile::get(name);
        let baseline = args_with_override(&profile, encoder, 33);
        let got = value_of(&baseline, flag)
            .unwrap_or_else(|| panic!("{name}: no {flag} in {baseline:?}"));
        assert_eq!(got, "33", "{name}: override must reach {flag}");
    }
}

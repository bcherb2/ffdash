mod ffmpeg_cmd;
mod ffmpeg_info;
mod log;
mod profile;
mod scan;
mod state;
mod types;

pub use ffmpeg_cmd::{
    build_ffmpeg_cmd, build_ffmpeg_cmd_with_profile, build_software_cmd, build_vaapi_cmd,
    encode_job, encode_job_with_callback, encode_job_with_callback_and_profile, format_ffmpeg_cmd,
    validate_vaapi_config,
};
pub use ffmpeg_info::{
    ffmpeg_version, ffprobe_version, parse_ffprobe_duration, probe_duration, vmaf_filter_available,
};
pub use log::write_debug_log;
pub use profile::{HwEncodingConfig, Profile, derive_output_path};
pub use scan::{build_job_from_path, build_job_queue, is_video_file, scan, scan_streaming};
pub use state::EncState;
pub use types::{JobStatus, ProgressParser, VideoJob};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    #[test]
    fn test_is_video_file() {
        assert!(is_video_file(Path::new("test.mp4")));
        assert!(is_video_file(Path::new("test.MP4")));
        assert!(is_video_file(Path::new("test.mkv")));
        assert!(is_video_file(Path::new("test.webm")));
        assert!(is_video_file(Path::new("test.mov")));
        assert!(is_video_file(Path::new("test.avi")));

        assert!(!is_video_file(Path::new("test.txt")));
        assert!(!is_video_file(Path::new("test.jpg")));
        assert!(!is_video_file(Path::new("test")));
    }

    #[test]
    fn test_parse_ffprobe_duration() {
        // Sample ffprobe JSON output
        let json = r#"{
            "format": {
                "filename": "test.mp4",
                "duration": "123.456",
                "size": "1024000"
            }
        }"#;

        let duration = parse_ffprobe_duration(json).expect("Failed to parse duration");
        assert_eq!(duration, 123.456);
    }

    #[test]
    fn test_parse_ffprobe_duration_integer() {
        let json = r#"{
            "format": {
                "duration": "60"
            }
        }"#;

        let duration = parse_ffprobe_duration(json).expect("Failed to parse duration");
        assert_eq!(duration, 60.0);
    }

    #[test]
    fn test_progress_parser_basic() {
        let mut parser = ProgressParser::new();

        parser.parse_line("out_time_us=5000000");
        assert_eq!(parser.out_time_us, 5_000_000);
        assert_eq!(parser.out_time_s(), 5.0);

        parser.parse_line("fps=30.5");
        assert_eq!(parser.fps, Some(30.5));

        parser.parse_line("speed=1.5x");
        assert_eq!(parser.speed, Some(1.5));

        parser.parse_line("bitrate=150.3kbits/s");
        assert_eq!(parser.bitrate_kbps, Some(150.3));

        parser.parse_line("total_size=1024000");
        assert_eq!(parser.total_size, Some(1024000));

        parser.parse_line("progress=end");
        assert!(parser.is_complete);
    }

    #[test]
    fn test_progress_percentage() {
        let mut parser = ProgressParser::new();
        parser.parse_line("out_time_us=5000000"); // 5 seconds

        // Test with 10 second duration
        assert_eq!(parser.progress_pct(Some(10.0)), 50.0);

        // Test with 5 second duration
        assert_eq!(parser.progress_pct(Some(5.0)), 100.0);

        // Test with no duration
        assert_eq!(parser.progress_pct(None), 0.0);
    }

    #[test]
    fn test_enc_state_serde_roundtrip() {
        let job1 = VideoJob::new(
            PathBuf::from("test1.mp4"),
            PathBuf::from("test1.vp9good.webm"),
            "vp9-good".to_string(),
        );
        let mut job2 = VideoJob::new(
            PathBuf::from("test2.mp4"),
            PathBuf::from("test2.vp9good.webm"),
            "vp9-good".to_string(),
        );
        job2.status = JobStatus::Done;
        job2.progress_pct = 100.0;

        let jobs = vec![job1.clone(), job2.clone()];
        let state = EncState::new(
            jobs.clone(),
            "vp9-good".to_string(),
            PathBuf::from("/test/path"),
        );

        // Serialize to JSON
        let json = serde_json::to_string(&state).expect("Failed to serialize");

        // Deserialize back
        let deserialized: EncState = serde_json::from_str(&json).expect("Failed to deserialize");

        // Verify
        assert_eq!(deserialized.jobs.len(), 2);
        assert_eq!(deserialized.jobs[0].input_path, job1.input_path);
        assert_eq!(deserialized.jobs[1].status, JobStatus::Done);
        assert_eq!(deserialized.selected_profile, "vp9-good");
        assert_eq!(deserialized.root_path, PathBuf::from("/test/path"));
    }

    #[test]
    fn test_build_job_queue_with_overwrite() {
        use std::fs;
        use tempfile::TempDir;

        // Create temp directory with test files
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Create 4 input video files
        let input_files: Vec<PathBuf> = (1..=4)
            .map(|i| {
                let path = dir_path.join(format!("video{}.mp4", i));
                fs::write(&path, b"fake video").unwrap();
                path
            })
            .collect();

        // Create output files for 3 of them (simulating previous run)
        // Note: vp9-good profile has no filename_pattern, so output is just {basename}.webm
        for i in 1..=3 {
            let output = dir_path.join(format!("video{}.webm", i));
            fs::write(&output, b"fake output").unwrap();
        }

        // Test with overwrite=false (should skip existing outputs)
        let jobs_no_overwrite =
            build_job_queue(input_files.clone(), "vp9-good", false, None, None, None);
        assert_eq!(jobs_no_overwrite.len(), 4);
        assert_eq!(
            jobs_no_overwrite[0].status,
            JobStatus::Skipped,
            "video1 output exists, should be Skipped"
        );
        assert_eq!(
            jobs_no_overwrite[1].status,
            JobStatus::Skipped,
            "video2 output exists, should be Skipped"
        );
        assert_eq!(
            jobs_no_overwrite[2].status,
            JobStatus::Skipped,
            "video3 output exists, should be Skipped"
        );
        assert_eq!(
            jobs_no_overwrite[3].status,
            JobStatus::Pending,
            "video4 output doesn't exist, should be Pending"
        );

        // Test with overwrite=true (should NOT skip any)
        let jobs_with_overwrite =
            build_job_queue(input_files.clone(), "vp9-good", true, None, None, None);
        assert_eq!(jobs_with_overwrite.len(), 4);
        assert_eq!(
            jobs_with_overwrite[0].status,
            JobStatus::Pending,
            "overwrite=true, all jobs should be Pending"
        );
        assert_eq!(
            jobs_with_overwrite[1].status,
            JobStatus::Pending,
            "overwrite=true, all jobs should be Pending"
        );
        assert_eq!(
            jobs_with_overwrite[2].status,
            JobStatus::Pending,
            "overwrite=true, all jobs should be Pending"
        );
        assert_eq!(
            jobs_with_overwrite[3].status,
            JobStatus::Pending,
            "overwrite=true, all jobs should be Pending"
        );
    }

    #[test]
    fn test_enc_state_with_overwrite_ignores_previous_state() {
        use std::fs;
        use tempfile::TempDir;

        // Create temp directory with test files
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Create 4 input video files
        let input_files: Vec<PathBuf> = (1..=4)
            .map(|i| {
                let path = dir_path.join(format!("video{}.mp4", i));
                fs::write(&path, b"fake video").unwrap();
                path
            })
            .collect();

        // Create output files for all 4 (simulating previous run)
        // Note: vp9-good profile has no filename_pattern, so output is just {basename}.webm
        for i in 1..=4 {
            let output = dir_path.join(format!("video{}.webm", i));
            fs::write(&output, b"fake output").unwrap();
        }

        // Simulate a previous run: create .enc_state and .enc_queue with 3 completed jobs
        let prev_jobs = build_job_queue(input_files.clone(), "vp9-good", false, None, None, None);
        let mut prev_state =
            EncState::new(prev_jobs, "vp9-good".to_string(), dir_path.to_path_buf());

        // Mark first 3 as Done
        for i in 0..3 {
            prev_state.jobs[i].status = JobStatus::Done;
            prev_state.jobs[i].progress_pct = 100.0;
        }

        // Save previous state
        prev_state.save(dir_path).unwrap();

        // Write .enc_queue with 3 completed entries
        let queue_content =
            "# VP9 Encode Queue\n# video1.mp4\n# video2.mp4\n# video3.mp4\nvideo4.mp4\n";
        fs::write(dir_path.join(".enc_queue"), queue_content).unwrap();

        // Now simulate starting a new run with overwrite=true
        let fresh_jobs = build_job_queue(input_files.clone(), "vp9-good", true, None, None, None);

        // All fresh jobs should be Pending (overwrite=true)
        assert_eq!(
            fresh_jobs[0].status,
            JobStatus::Pending,
            "Fresh job with overwrite=true should be Pending"
        );
        assert_eq!(
            fresh_jobs[1].status,
            JobStatus::Pending,
            "Fresh job with overwrite=true should be Pending"
        );
        assert_eq!(
            fresh_jobs[2].status,
            JobStatus::Pending,
            "Fresh job with overwrite=true should be Pending"
        );
        assert_eq!(
            fresh_jobs[3].status,
            JobStatus::Pending,
            "Fresh job with overwrite=true should be Pending"
        );

        // Create fresh EncState (simulating what should happen with overwrite=true)
        let fresh_state = EncState::new(
            fresh_jobs.clone(),
            "vp9-good".to_string(),
            dir_path.to_path_buf(),
        );

        // All jobs in fresh state should be Pending
        assert_eq!(
            fresh_state.jobs[0].status,
            JobStatus::Pending,
            "Fresh EncState job should be Pending"
        );
        assert_eq!(
            fresh_state.jobs[1].status,
            JobStatus::Pending,
            "Fresh EncState job should be Pending"
        );
        assert_eq!(
            fresh_state.jobs[2].status,
            JobStatus::Pending,
            "Fresh EncState job should be Pending"
        );
        assert_eq!(
            fresh_state.jobs[3].status,
            JobStatus::Pending,
            "Fresh EncState job should be Pending"
        );

        // Now simulate what would happen if we incorrectly loaded the old state
        let mut loaded_state = EncState::load(dir_path).unwrap();
        loaded_state.load_queue_status(dir_path).unwrap();

        // The loaded state would have 3 Done and 1 Pending (wrong for overwrite=true!)
        assert_eq!(
            loaded_state.jobs[0].status,
            JobStatus::Done,
            "Loaded state has old status"
        );
        assert_eq!(
            loaded_state.jobs[1].status,
            JobStatus::Done,
            "Loaded state has old status"
        );
        assert_eq!(
            loaded_state.jobs[2].status,
            JobStatus::Done,
            "Loaded state has old status"
        );
        assert_eq!(
            loaded_state.jobs[3].status,
            JobStatus::Skipped,
            "Loaded state has old status"
        );
    }

    #[test]
    fn test_build_job_queue_with_custom_output_dir() {
        use std::fs;
        use tempfile::TempDir;

        // Create temp directory with test files
        let temp_dir = TempDir::new().unwrap();
        let input_dir = temp_dir.path().join("input");
        let output_dir = temp_dir.path().join("output");

        fs::create_dir(&input_dir).unwrap();
        fs::create_dir(&output_dir).unwrap();

        // Create 3 input video files in input directory
        let input_files: Vec<PathBuf> = (1..=3)
            .map(|i| {
                let path = input_dir.join(format!("video{}.mp4", i));
                fs::write(&path, b"fake video").unwrap();
                path
            })
            .collect();

        // Build job queue with custom output directory
        let output_dir_str = output_dir.to_str().unwrap();
        let jobs = build_job_queue(
            input_files.clone(),
            "vp9-good",
            false,
            Some(output_dir_str),
            None,
            None,
        );

        assert_eq!(jobs.len(), 3);

        // Verify all output paths are in the custom output directory
        for (i, job) in jobs.iter().enumerate() {
            let expected_output = output_dir.join(format!("video{}.webm", i + 1));
            assert_eq!(
                job.output_path,
                expected_output,
                "Job {} output should be in custom output directory",
                i + 1
            );
            assert_eq!(job.status, JobStatus::Pending, "All jobs should be Pending");
        }

        // Now create one output file in the custom directory
        fs::write(output_dir.join("video1.webm"), b"fake output").unwrap();

        // Rebuild queue - should skip the file that now exists
        let jobs_after = build_job_queue(
            input_files.clone(),
            "vp9-good",
            false,
            Some(output_dir_str),
            None,
            None,
        );
        assert_eq!(
            jobs_after[0].status,
            JobStatus::Skipped,
            "video1 output exists in custom dir, should be Skipped"
        );
        assert_eq!(
            jobs_after[1].status,
            JobStatus::Pending,
            "video2 output doesn't exist, should be Pending"
        );
        assert_eq!(
            jobs_after[2].status,
            JobStatus::Pending,
            "video3 output doesn't exist, should be Pending"
        );
    }

    #[test]
    fn test_vaapi_command_no_invalid_filter() {
        use super::{HwEncodingConfig, Profile};
        use std::path::PathBuf;

        let hw_config = HwEncodingConfig::default();

        let mut profile = Profile::get("vp9-good");
        profile.audio_codec = "libopus".to_string();
        profile.audio_bitrate = 128;
        profile.crf = 31;
        profile.video_target_bitrate = 0; // CQP mode

        let job = VideoJob {
            id: uuid::Uuid::new_v4(),
            input_path: PathBuf::from("/tmp/test.mp4"),
            output_path: PathBuf::from("/tmp/test.webm"),
            profile: "test".to_string(),
            status: JobStatus::Pending,
            overwrite: true,
            progress_pct: 0.0,
            duration_s: None,
            out_time_s: 0.0,
            fps: None,
            speed: None,
            smoothed_speed: None,
            bitrate_kbps: None,
            size_bytes: None,
            started_at: None,
            last_speed_update: None,
            displayed_eta_seconds: None,
            attempts: 0,
            last_error: None,
        };

        let cmd = build_vaapi_cmd(&job, &profile, &hw_config);
        let args: Vec<String> = cmd
            .get_args()
            .map(|s| s.to_string_lossy().to_string())
            .collect();
        let full_cmd = args.join(" ");

        // Test 1: Should NOT contain the broken filter chain
        assert!(
            !full_cmd.contains("format=nv12,hwupload"),
            "VAAPI command should not contain 'format=nv12,hwupload' filter when using hwaccel_output_format"
        );

        // Test 2: Should contain proper hwaccel setup
        assert!(
            full_cmd.contains("-hwaccel vaapi"),
            "VAAPI command should contain '-hwaccel vaapi'"
        );
        assert!(
            full_cmd.contains("-hwaccel_output_format vaapi"),
            "VAAPI command should contain '-hwaccel_output_format vaapi'"
        );

        // Test 3: Should contain vp9_vaapi encoder
        assert!(
            full_cmd.contains("vp9_vaapi"),
            "VAAPI command should use vp9_vaapi encoder"
        );

        // Test 4: Should force CQP mode (rc_mode = 1) for reliability
        // hw_config.global_quality = 70 (default) passed directly to FFmpeg in CQP mode
        assert!(
            full_cmd.contains("-rc_mode 1"),
            "VAAPI command should force CQP (rc_mode 1) as default mode"
        );
        assert!(
            full_cmd.contains("-global_quality 70"),
            "VAAPI command should use global_quality 70 (passed directly)"
        );

        // Test 5: CQP mode should NOT contain bitrate parameters
        assert!(
            !full_cmd.contains("-b:v"),
            "VAAPI CQP mode should not contain -b:v parameter"
        );
        assert!(
            !full_cmd.contains("-maxrate"),
            "VAAPI CQP mode should not contain -maxrate parameter"
        );
    }

    #[test]
    fn test_vaapi_command_no_invalid_vbr_flag() {
        use super::{HwEncodingConfig, Profile};
        use std::path::PathBuf;

        let hw_config = HwEncodingConfig::default();

        let mut profile = Profile::get("vp9-good");
        profile.crf = 31;
        profile.video_target_bitrate = 0; // CQP mode
        profile.audio_codec = "libvorbis".to_string(); // Use libvorbis for CQP (libopus incompatible)

        let job = VideoJob {
            id: uuid::Uuid::new_v4(),
            input_path: PathBuf::from("/tmp/test.mp4"),
            output_path: PathBuf::from("/tmp/test.webm"),
            profile: "test".to_string(),
            status: JobStatus::Pending,
            overwrite: true,
            progress_pct: 0.0,
            duration_s: None,
            out_time_s: 0.0,
            fps: None,
            speed: None,
            smoothed_speed: None,
            bitrate_kbps: None,
            size_bytes: None,
            started_at: None,
            last_speed_update: None,
            displayed_eta_seconds: None,
            attempts: 0,
            last_error: None,
        };

        let cmd = build_vaapi_cmd(&job, &profile, &hw_config);
        let args: Vec<String> = cmd
            .get_args()
            .map(|s| s.to_string_lossy().to_string())
            .collect();
        let full_cmd = args.join(" ");

        // Test 1: Should NOT contain libopus-specific VBR parameters when using libvorbis
        assert!(
            !full_cmd.contains("-vbr"),
            "VAAPI with libvorbis should not contain libopus '-vbr' flag"
        );
        assert!(
            !full_cmd.contains("-compression_level 10"),
            "VAAPI with libvorbis should not contain libopus '-compression_level 10'"
        );

        // Test 2: Should contain VAAPI video compression_level (always present for vp9_vaapi)
        assert!(
            full_cmd.contains("-compression_level 4"),
            "VAAPI should contain video encoder compression_level"
        );

        // Test 3: Should use libvorbis audio codec
        assert!(
            full_cmd.contains("libvorbis"),
            "VAAPI should use libvorbis audio codec"
        );
        assert!(
            full_cmd.contains("-b:a 128k"),
            "VAAPI command should set audio bitrate"
        );
    }

    #[test]
    fn test_vaapi_command_vbr_mode() {
        use super::{HwEncodingConfig, Profile};
        use std::path::PathBuf;

        let mut hw_config = HwEncodingConfig::default();
        hw_config.rc_mode = 3; // Explicitly set VBR mode

        let mut profile = Profile::get("vp9-good");
        profile.audio_codec = "libopus".to_string();
        profile.audio_bitrate = 128;
        profile.crf = 31;
        profile.video_target_bitrate = 5000; // VBR bitrate target
        profile.video_max_bitrate = 8000;
        profile.video_bufsize = 10000;

        let job = VideoJob {
            id: uuid::Uuid::new_v4(),
            input_path: PathBuf::from("/tmp/test.mp4"),
            output_path: PathBuf::from("/tmp/test.webm"),
            profile: "test".to_string(),
            status: JobStatus::Pending,
            overwrite: true,
            progress_pct: 0.0,
            duration_s: None,
            out_time_s: 0.0,
            fps: None,
            speed: None,
            smoothed_speed: None,
            bitrate_kbps: None,
            size_bytes: None,
            started_at: None,
            last_speed_update: None,
            displayed_eta_seconds: None,
            attempts: 0,
            last_error: None,
        };

        let cmd = build_vaapi_cmd(&job, &profile, &hw_config);
        let args: Vec<String> = cmd
            .get_args()
            .map(|s| s.to_string_lossy().to_string())
            .collect();
        let full_cmd = args.join(" ");

        // Test 1: Even when VBR is requested, we force CQP (rc_mode = 1) for reliability
        assert!(
            full_cmd.contains("-rc_mode 1"),
            "VAAPI command should force CQP (rc_mode 1) even when VBR is requested"
        );

        // Test 2: CQP mode should NOT add bitrate settings
        assert!(
            !full_cmd.contains("-b:v"),
            "VAAPI CQP mode should not set video bitrate"
        );
        assert!(
            !full_cmd.contains("-maxrate"),
            "VAAPI CQP mode should not set max bitrate"
        );
        assert!(
            !full_cmd.contains("-bufsize"),
            "VAAPI CQP mode should not set buffer size"
        );

        // Test 3: Should include global_quality in forced CQP mode
        assert!(
            full_cmd.contains("-global_quality 70"),
            "VAAPI command should include global_quality when forcing CQP"
        );
    }
}

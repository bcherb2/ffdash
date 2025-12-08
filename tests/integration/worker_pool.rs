// Tests for worker pool and message handling

use ffdash::engine::{
    JobStatus, VideoJob,
    worker::{WorkerMessage, WorkerPool},
};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

#[test]
fn test_worker_pool_initialization() {
    let pool = WorkerPool::new(3);
    assert_eq!(
        pool.active_count(),
        0,
        "Pool should start with 0 active workers"
    );
    assert_eq!(
        pool.max_workers(),
        3,
        "Pool should have max_workers set to 3"
    );
    assert!(
        pool.can_spawn(),
        "Pool should allow spawning when active < max"
    );
}

#[test]
fn test_worker_pool_dynamic_max_workers() {
    let pool = WorkerPool::new(2);

    // Initial state
    assert_eq!(pool.max_workers(), 2, "Initial max should be 2");
    assert!(
        pool.can_spawn(),
        "Should be able to spawn with 0 active and max 2"
    );

    // Increase max_workers
    pool.set_max_workers(5);
    assert_eq!(pool.max_workers(), 5, "Max should be updated to 5");
    assert!(pool.can_spawn(), "Should still be able to spawn");

    // Decrease max_workers
    pool.set_max_workers(1);
    assert_eq!(pool.max_workers(), 1, "Max should be updated to 1");

    // With 0 active, should still be able to spawn
    assert!(
        pool.can_spawn(),
        "Should be able to spawn 1 worker when max is 1"
    );
}

#[test]
fn test_worker_pool_can_spawn_limit() {
    let pool = Arc::new(WorkerPool::new(2));

    // Create dummy jobs
    let _job1 = VideoJob::new(
        PathBuf::from("test1.mp4"),
        PathBuf::from("out1.webm"),
        "test".to_string(),
    );
    let _job2 = VideoJob::new(
        PathBuf::from("test2.mp4"),
        PathBuf::from("out2.webm"),
        "test".to_string(),
    );

    // Note: We can't actually test spawning without real video files
    // This test just verifies the pool structure
    assert!(pool.can_spawn(), "Should be able to spawn first worker");
}

#[test]
fn test_worker_message_types() {
    use uuid::Uuid;

    let job_id = Uuid::new_v4();

    // Test JobStarted message creation
    let msg = WorkerMessage::JobStarted { job_id };
    match msg {
        WorkerMessage::JobStarted { job_id: id } => assert_eq!(id, job_id),
        _ => panic!("Wrong message type"),
    }

    // Test ProgressUpdate message
    let msg = WorkerMessage::ProgressUpdate {
        job_id,
        progress_pct: 50.0,
        out_time_s: 30.0,
        fps: Some(60.0),
        speed: Some(1.5),
        bitrate_kbps: Some(2500.0),
        size_bytes: Some(1024000),
    };
    match msg {
        WorkerMessage::ProgressUpdate {
            progress_pct,
            speed,
            ..
        } => {
            assert_eq!(progress_pct, 50.0);
            assert_eq!(speed, Some(1.5));
        }
        _ => panic!("Wrong message type"),
    }

    // Test JobCompleted message
    let msg = WorkerMessage::JobCompleted { job_id };
    match msg {
        WorkerMessage::JobCompleted { job_id: id } => assert_eq!(id, job_id),
        _ => panic!("Wrong message type"),
    }

    // Test JobFailed message
    let msg = WorkerMessage::JobFailed {
        job_id,
        error: "Test error".to_string(),
    };
    match msg {
        WorkerMessage::JobFailed { error, .. } => assert_eq!(error, "Test error"),
        _ => panic!("Wrong message type"),
    }

    // Test WorkerIdle message
    let msg = WorkerMessage::WorkerIdle { worker_id: 0 };
    match msg {
        WorkerMessage::WorkerIdle { worker_id } => assert_eq!(worker_id, 0),
        _ => panic!("Wrong message type"),
    }
}

#[test]
fn test_job_status_transitions() {
    let mut job = VideoJob::new(
        PathBuf::from("test.mp4"),
        PathBuf::from("out.webm"),
        "test".to_string(),
    );

    assert_eq!(job.status, JobStatus::Pending, "New job should be Pending");

    job.status = JobStatus::Running;
    assert_eq!(job.status, JobStatus::Running);

    job.status = JobStatus::Done;
    assert_eq!(job.status, JobStatus::Done);

    job.status = JobStatus::Failed;
    assert_eq!(job.status, JobStatus::Failed);
}

#[test]
fn test_job_progress_tracking() {
    let mut job = VideoJob::new(
        PathBuf::from("test.mp4"),
        PathBuf::from("out.webm"),
        "test".to_string(),
    );

    // Simulate progress updates
    job.progress_pct = 25.0;
    job.out_time_s = 15.0;
    job.speed = Some(1.2);
    job.fps = Some(30.0);

    assert_eq!(job.progress_pct, 25.0);
    assert_eq!(job.out_time_s, 15.0);
    assert_eq!(job.speed, Some(1.2));
    assert_eq!(job.fps, Some(30.0));

    // Simulate completion
    job.status = JobStatus::Done;
    job.progress_pct = 100.0;

    assert_eq!(job.status, JobStatus::Done);
    assert_eq!(job.progress_pct, 100.0);
}

#[test]
fn test_worker_pool_receiver() {
    let pool = WorkerPool::new(1);
    let receiver = pool.receiver();

    // Try to receive with timeout (should timeout immediately as no workers are running)
    let result = receiver.recv_timeout(Duration::from_millis(10));
    assert!(result.is_err(), "Should timeout when no messages available");
}

#[cfg(test)]
mod queue_persistence_tests {
    use super::*;
    use ffdash::engine::EncState;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_enc_queue_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create jobs with different statuses
        let mut jobs = vec![
            VideoJob::new(
                root.join("video1.mp4"),
                root.join("out1.webm"),
                "test".to_string(),
            ),
            VideoJob::new(
                root.join("video2.mp4"),
                root.join("out2.webm"),
                "test".to_string(),
            ),
            VideoJob::new(
                root.join("video3.mp4"),
                root.join("out3.webm"),
                "test".to_string(),
            ),
        ];

        jobs[0].status = JobStatus::Done;
        jobs[1].status = JobStatus::Running;
        jobs[2].status = JobStatus::Pending;

        let enc_state = EncState::new(jobs, "test".to_string(), root.to_path_buf());

        // Save queue status
        enc_state.save_queue_status(root).unwrap();

        // Verify .enc_queue file exists
        let queue_file = root.join(".enc_queue");
        assert!(queue_file.exists(), ".enc_queue file should be created");

        // Read and verify contents
        let contents = fs::read_to_string(&queue_file).unwrap();
        assert!(
            contents.contains("# video1.mp4"),
            "Completed job should have # prefix"
        );
        assert!(
            contents.contains("video2.mp4") && !contents.contains("# video2.mp4\n"),
            "Running job should not have # prefix"
        );
        assert!(
            contents.contains("video3.mp4") && !contents.contains("# video3.mp4\n"),
            "Pending job should not have # prefix"
        );
    }

    #[test]
    fn test_enc_queue_load_completion_status() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create .enc_queue file manually
        let queue_content = r#"# VP9 Encoding Queue - Lines with # prefix are completed
# video1.mp4
video2.mp4
# video3.mp4 (skipped - output exists)
"#;
        fs::write(root.join(".enc_queue"), queue_content).unwrap();

        // Create jobs (all pending initially)
        let jobs = vec![
            VideoJob::new(
                root.join("video1.mp4"),
                root.join("out1.webm"),
                "test".to_string(),
            ),
            VideoJob::new(
                root.join("video2.mp4"),
                root.join("out2.webm"),
                "test".to_string(),
            ),
            VideoJob::new(
                root.join("video3.mp4"),
                root.join("out3.webm"),
                "test".to_string(),
            ),
        ];

        let mut enc_state = EncState::new(jobs, "test".to_string(), root.to_path_buf());

        // Load completion status
        enc_state.load_queue_status(root).unwrap();

        // Verify statuses were updated
        assert_eq!(
            enc_state.jobs[0].status,
            JobStatus::Done,
            "video1 should be marked Done"
        );
        assert_eq!(
            enc_state.jobs[1].status,
            JobStatus::Pending,
            "video2 should remain Pending"
        );
        assert_eq!(
            enc_state.jobs[2].status,
            JobStatus::Done,
            "video3 should be marked Done"
        );

        assert_eq!(
            enc_state.jobs[0].progress_pct, 100.0,
            "Completed job should have 100% progress"
        );
    }
}

// Integration tests for ffdash
// This file serves as the main entry point for integration tests

mod common;

// Include all integration test modules
#[path = "integration/ffmpeg_commands.rs"]
mod ffmpeg_commands;

#[path = "integration/profile_workflows.rs"]
mod profile_workflows;

#[path = "integration/config_state.rs"]
mod config_state;

#[path = "integration/ffmpeg_e2e.rs"]
mod ffmpeg_e2e;

#[path = "integration/ffmpeg_e2e_proptest.rs"]
mod ffmpeg_e2e_proptest;

#[path = "integration/worker_pool.rs"]
mod worker_pool;

#[path = "integration/fps_duration_preservation.rs"]
mod fps_duration_preservation;

#[path = "integration/parameter_coverage.rs"]
mod parameter_coverage;

#[path = "integration/vaapi_vs_software_parity.rs"]
mod vaapi_vs_software_parity;

// Realistic integration tests for VAAPI driver detection

use ffdash::engine::hardware;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_driver_found_in_first_location() {
    // Create temp directory with driver in first search location
    let temp = TempDir::new().unwrap();
    let first_dir = temp.path().join("first");
    fs::create_dir_all(&first_dir).unwrap();
    fs::write(first_dir.join("iHD_drv_video.so"), "mock driver").unwrap();

    let second_dir = temp.path().join("second");
    fs::create_dir_all(&second_dir).unwrap();
    // No driver in second location

    let search_paths = [first_dir.to_str().unwrap(), second_dir.to_str().unwrap()];

    let result = hardware::detect_vaapi_driver_path_custom(&search_paths);

    assert_eq!(result, Some(first_dir.to_str().unwrap().to_string()));
}

#[test]
fn test_driver_found_in_second_location() {
    // Test priority: should find in second if not in first
    let temp = TempDir::new().unwrap();

    let first_dir = temp.path().join("first");
    fs::create_dir_all(&first_dir).unwrap();
    // No driver in first location

    let second_dir = temp.path().join("second");
    fs::create_dir_all(&second_dir).unwrap();
    fs::write(second_dir.join("iHD_drv_video.so"), "mock driver").unwrap();

    let search_paths = [first_dir.to_str().unwrap(), second_dir.to_str().unwrap()];

    let result = hardware::detect_vaapi_driver_path_custom(&search_paths);

    assert_eq!(result, Some(second_dir.to_str().unwrap().to_string()));
}

#[test]
fn test_no_driver_found() {
    // Test when no driver exists in any location
    let temp = TempDir::new().unwrap();

    let first_dir = temp.path().join("first");
    fs::create_dir_all(&first_dir).unwrap();

    let second_dir = temp.path().join("second");
    fs::create_dir_all(&second_dir).unwrap();

    let search_paths = [first_dir.to_str().unwrap(), second_dir.to_str().unwrap()];

    let result = hardware::detect_vaapi_driver_path_custom(&search_paths);

    assert_eq!(result, None);
}

#[test]
fn test_priority_first_over_second() {
    // Test that first location is preferred even if both have drivers
    let temp = TempDir::new().unwrap();

    let first_dir = temp.path().join("first");
    fs::create_dir_all(&first_dir).unwrap();
    fs::write(first_dir.join("iHD_drv_video.so"), "first driver").unwrap();

    let second_dir = temp.path().join("second");
    fs::create_dir_all(&second_dir).unwrap();
    fs::write(second_dir.join("iHD_drv_video.so"), "second driver").unwrap();

    let search_paths = [first_dir.to_str().unwrap(), second_dir.to_str().unwrap()];

    let result = hardware::detect_vaapi_driver_path_custom(&search_paths);

    // Should return first, not second
    assert_eq!(result, Some(first_dir.to_str().unwrap().to_string()));
}

#[test]
fn test_nonexistent_directory() {
    // Test when search path doesn't exist at all
    let temp = TempDir::new().unwrap();

    let nonexistent = temp.path().join("does_not_exist");
    // Don't create the directory

    let existing_dir = temp.path().join("exists");
    fs::create_dir_all(&existing_dir).unwrap();
    fs::write(existing_dir.join("iHD_drv_video.so"), "driver").unwrap();

    let search_paths = [
        nonexistent.to_str().unwrap(),
        existing_dir.to_str().unwrap(),
    ];

    let result = hardware::detect_vaapi_driver_path_custom(&search_paths);

    // Should skip nonexistent and find in existing
    assert_eq!(result, Some(existing_dir.to_str().unwrap().to_string()));
}

#[test]
fn test_wrong_filename_ignored() {
    // Test that wrong filenames are ignored
    let temp = TempDir::new().unwrap();

    let dir = temp.path().join("driver_dir");
    fs::create_dir_all(&dir).unwrap();

    // Create wrong filename
    fs::write(dir.join("wrong_driver.so"), "driver").unwrap();
    fs::write(dir.join("iHD_drv_video.txt"), "driver").unwrap();
    // Not the correct iHD_drv_video.so

    let search_paths = [dir.to_str().unwrap()];

    let result = hardware::detect_vaapi_driver_path_custom(&search_paths);

    assert_eq!(result, None);
}

#[test]
fn test_empty_file_is_valid() {
    // Test that even empty driver file is detected
    let temp = TempDir::new().unwrap();

    let dir = temp.path().join("driver_dir");
    fs::create_dir_all(&dir).unwrap();

    // Create empty driver file (just check existence)
    fs::write(dir.join("iHD_drv_video.so"), "").unwrap();

    let search_paths = [dir.to_str().unwrap()];

    let result = hardware::detect_vaapi_driver_path_custom(&search_paths);

    assert_eq!(result, Some(dir.to_str().unwrap().to_string()));
}

#[test]
fn test_symlink_driver() {
    // Test that symlinks are followed
    let temp = TempDir::new().unwrap();

    let driver_dir = temp.path().join("driver_dir");
    fs::create_dir_all(&driver_dir).unwrap();

    // Create actual driver file
    let real_driver = temp.path().join("real_driver.so");
    fs::write(&real_driver, "real driver").unwrap();

    // Create symlink (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let link_path = driver_dir.join("iHD_drv_video.so");
        symlink(&real_driver, &link_path).unwrap();

        let search_paths = [driver_dir.to_str().unwrap()];
        let result = hardware::detect_vaapi_driver_path_custom(&search_paths);

        assert_eq!(result, Some(driver_dir.to_str().unwrap().to_string()));
    }
}

#[test]
fn test_ubuntu_specific_path_priority() {
    // Realistic test: Ubuntu path should be checked before generic
    let temp = TempDir::new().unwrap();

    let generic = temp.path().join("usr/lib/dri");
    fs::create_dir_all(&generic).unwrap();
    fs::write(generic.join("iHD_drv_video.so"), "generic").unwrap();

    let ubuntu = temp.path().join("usr/lib/x86_64-linux-gnu/dri");
    fs::create_dir_all(&ubuntu).unwrap();
    fs::write(ubuntu.join("iHD_drv_video.so"), "ubuntu").unwrap();

    // Ubuntu path should be FIRST in search order
    let search_paths = [ubuntu.to_str().unwrap(), generic.to_str().unwrap()];

    let result = hardware::detect_vaapi_driver_path_custom(&search_paths);

    // Should find Ubuntu-specific path first
    assert_eq!(result, Some(ubuntu.to_str().unwrap().to_string()));
}

#[test]
fn test_multiple_search_paths_realistic() {
    // Simulate realistic multi-distro search
    let temp = TempDir::new().unwrap();

    // Create multiple distro-style paths
    let ubuntu = temp.path().join("usr/lib/x86_64-linux-gnu/dri");
    fs::create_dir_all(&ubuntu).unwrap();

    let generic = temp.path().join("usr/lib/dri");
    fs::create_dir_all(&generic).unwrap();

    let rhel = temp.path().join("usr/lib64/dri");
    fs::create_dir_all(&rhel).unwrap();

    // Only RHEL has the driver
    fs::write(rhel.join("iHD_drv_video.so"), "rhel driver").unwrap();

    let search_paths = [
        ubuntu.to_str().unwrap(),
        generic.to_str().unwrap(),
        rhel.to_str().unwrap(),
    ];

    let result = hardware::detect_vaapi_driver_path_custom(&search_paths);

    assert_eq!(result, Some(rhel.to_str().unwrap().to_string()));
}

#[cfg(target_os = "linux")]
#[test]
fn test_real_system_driver_detection() {
    // Test on actual Linux system (if available)
    let result = hardware::detect_vaapi_driver_path();

    // Don't fail if not found, just report
    match result {
        Some(path) => {
            println!("✓ Found VAAPI driver at: {}", path);
            // Verify the file actually exists
            let driver_file = std::path::Path::new(&path).join("iHD_drv_video.so");
            assert!(
                driver_file.exists(),
                "Driver file should exist at reported path"
            );
        }
        None => {
            println!("✗ No VAAPI driver found on this system");
            println!("  This is expected if:");
            println!("    - Not on Intel hardware");
            println!("    - Running in CI/container without drivers");
            println!("    - Drivers not installed");
        }
    }
}

// ============================================================
// Tests for LIBVA_DRIVERS_PATH environment variable priority
// ============================================================

#[test]
fn test_env_path_takes_priority_when_valid() {
    // When LIBVA_DRIVERS_PATH is set and driver exists there, use it
    let temp = TempDir::new().unwrap();

    let env_dir = temp.path().join("env_path");
    fs::create_dir_all(&env_dir).unwrap();
    fs::write(env_dir.join("iHD_drv_video.so"), "env driver").unwrap();

    let search_dir = temp.path().join("search_path");
    fs::create_dir_all(&search_dir).unwrap();
    fs::write(search_dir.join("iHD_drv_video.so"), "search driver").unwrap();

    let search_paths = [search_dir.to_str().unwrap()];

    // Env path should be used, not search path
    let result =
        hardware::detect_vaapi_driver_path_with_env(Some(env_dir.to_str().unwrap()), &search_paths);

    assert_eq!(result, Some(env_dir.to_str().unwrap().to_string()));
}

#[test]
fn test_env_path_fallback_when_invalid() {
    // When LIBVA_DRIVERS_PATH is set but driver doesn't exist, fall back to search
    let temp = TempDir::new().unwrap();

    let env_dir = temp.path().join("env_path");
    fs::create_dir_all(&env_dir).unwrap();
    // No driver file in env path!

    let search_dir = temp.path().join("search_path");
    fs::create_dir_all(&search_dir).unwrap();
    fs::write(search_dir.join("iHD_drv_video.so"), "search driver").unwrap();

    let search_paths = [search_dir.to_str().unwrap()];

    // Should fall back to search path since env path has no driver
    let result =
        hardware::detect_vaapi_driver_path_with_env(Some(env_dir.to_str().unwrap()), &search_paths);

    assert_eq!(result, Some(search_dir.to_str().unwrap().to_string()));
}

#[test]
fn test_no_env_uses_search_paths() {
    // When LIBVA_DRIVERS_PATH is not set, use search paths
    let temp = TempDir::new().unwrap();

    let search_dir = temp.path().join("search_path");
    fs::create_dir_all(&search_dir).unwrap();
    fs::write(search_dir.join("iHD_drv_video.so"), "search driver").unwrap();

    let search_paths = [search_dir.to_str().unwrap()];

    // No env path, should use search
    let result = hardware::detect_vaapi_driver_path_with_env(None, &search_paths);

    assert_eq!(result, Some(search_dir.to_str().unwrap().to_string()));
}

#[test]
fn test_env_path_nonexistent_directory() {
    // When LIBVA_DRIVERS_PATH points to nonexistent dir, fall back to search
    let temp = TempDir::new().unwrap();

    let nonexistent_env = temp.path().join("does_not_exist");
    // Don't create this directory

    let search_dir = temp.path().join("search_path");
    fs::create_dir_all(&search_dir).unwrap();
    fs::write(search_dir.join("iHD_drv_video.so"), "search driver").unwrap();

    let search_paths = [search_dir.to_str().unwrap()];

    let result = hardware::detect_vaapi_driver_path_with_env(
        Some(nonexistent_env.to_str().unwrap()),
        &search_paths,
    );

    assert_eq!(result, Some(search_dir.to_str().unwrap().to_string()));
}

#[test]
fn test_both_env_and_search_empty() {
    // When neither env nor search paths have driver, return None
    let temp = TempDir::new().unwrap();

    let env_dir = temp.path().join("env_path");
    fs::create_dir_all(&env_dir).unwrap();

    let search_dir = temp.path().join("search_path");
    fs::create_dir_all(&search_dir).unwrap();

    let search_paths = [search_dir.to_str().unwrap()];

    let result =
        hardware::detect_vaapi_driver_path_with_env(Some(env_dir.to_str().unwrap()), &search_paths);

    assert_eq!(result, None);
}

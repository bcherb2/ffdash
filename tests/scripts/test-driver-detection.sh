#!/bin/bash
# Test if VAAPI driver detection is working in the app

set -e

echo "=== Testing VAAPI Driver Detection ==="
echo ""

# Build the app
echo "1. Building app..."
cargo build --release 2>&1 | tail -1

# Create a simple Rust test program to check detection
cat > /tmp/test_detection.rs << 'EOF'
use std::path::Path;

fn detect_vaapi_driver_path() -> Option<String> {
    let search_paths = [
        "/usr/lib/x86_64-linux-gnu/dri",
        "/usr/local/lib/x86_64-linux-gnu/dri",
        "/usr/lib/dri",
        "/usr/local/lib/dri",
        "/usr/lib64/dri",
        "/usr/local/lib64/dri",
    ];

    for path in &search_paths {
        let driver_path = Path::new(path).join("iHD_drv_video.so");
        println!("  Checking: {}", driver_path.display());
        if driver_path.exists() {
            println!("  ✓ Found!");
            return Some(path.to_string());
        } else {
            println!("  ✗ Not found");
        }
    }
    None
}

fn main() {
    println!("Testing VAAPI driver detection logic:");
    match detect_vaapi_driver_path() {
        Some(path) => {
            println!("\n✓ Driver detected at: {}", path);
            println!("  Full path: {}/iHD_drv_video.so", path);
        }
        None => {
            println!("\n✗ No driver found!");
            println!("\nSearching entire system:");
            std::process::Command::new("find")
                .args(&["/usr", "-name", "iHD_drv_video.so", "2>/dev/null"])
                .status()
                .ok();
        }
    }
}
EOF

echo ""
echo "2. Testing detection logic..."
rustc /tmp/test_detection.rs -o /tmp/test_detection
/tmp/test_detection
rm /tmp/test_detection.rs /tmp/test_detection

echo ""
echo "3. What the app would actually do:"
echo "   When encoding, it would set:"
echo "   LIBVA_DRIVERS_PATH=$(cargo run --release --bin ffdash -- 2>&1 | grep 'LIBVA_DRIVERS_PATH' || echo 'NOT SET')"

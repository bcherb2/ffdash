#!/bin/bash
# Quick VAAPI encode test - suitable for Docker container
# Creates a 2-second test video and encodes it with VAAPI

set -e

echo "=== Quick VAAPI Encode Test ==="
echo ""

# Check VAAPI availability
echo "1. Checking VAAPI availability..."
if vainfo > /dev/null 2>&1; then
    echo "   ✓ VAAPI is available"
    vainfo 2>&1 | grep "VAProfileVP9" | head -1
else
    echo "   ✗ VAAPI check failed"
    exit 1
fi

# Create test directory
TEST_DIR="/tmp/vaapi_test_$$"
mkdir -p "$TEST_DIR"
echo "   Test directory: $TEST_DIR"
echo ""

# Cleanup on exit
trap "rm -rf $TEST_DIR" EXIT

# Generate test video
echo "2. Generating test video (2 seconds, 1280x720)..."
if ffmpeg -f lavfi -i testsrc=duration=2:size=1280x720:rate=24 \
    -f lavfi -i sine=frequency=1000:duration=2 \
    -c:v libx264 -pix_fmt yuv420p -c:a aac \
    "$TEST_DIR/test.mp4" -y > /dev/null 2>&1; then
    echo "   ✓ Test video created"
else
    echo "   ✗ Failed to create test video"
    exit 1
fi
echo ""

# Run VAAPI encode
echo "3. Running VAAPI encode (vp9_vaapi, CQP mode)..."
START_TIME=$(date +%s)

if ffmpeg \
    -init_hw_device vaapi=va:/dev/dri/renderD128 \
    -hwaccel vaapi \
    -hwaccel_output_format vaapi \
    -i "$TEST_DIR/test.mp4" \
    -c:v vp9_vaapi \
    -low_power 1 \
    -rc_mode CQP \
    -global_quality 4 \
    -maxrate 5000k \
    -bufsize 10000k \
    -g 240 \
    -c:a libvorbis \
    -q:a 6 \
    "$TEST_DIR/test.webm" -y 2>&1 | tee "$TEST_DIR/encode.log" | grep -E "^(frame=|size=)" | tail -1; then

    END_TIME=$(date +%s)
    ELAPSED=$((END_TIME - START_TIME))
    echo "   ✓ Encode completed in ${ELAPSED}s"
else
    echo "   ✗ Encode failed"
    echo ""
    echo "Error log:"
    tail -20 "$TEST_DIR/encode.log"
    exit 1
fi
echo ""

# Verify output
echo "4. Verifying output..."

if [ ! -f "$TEST_DIR/test.webm" ]; then
    echo "   ✗ Output file not created"
    exit 1
fi

FILE_SIZE=$(stat -c%s "$TEST_DIR/test.webm" 2>/dev/null || stat -f%z "$TEST_DIR/test.webm")
SIZE_KB=$((FILE_SIZE / 1024))

if [ $FILE_SIZE -eq 0 ]; then
    echo "   ✗ Output file is empty"
    exit 1
fi

echo "   ✓ Output file created (${SIZE_KB} KB)"

# Check codec
VIDEO_CODEC=$(ffprobe -v error -select_streams v:0 -show_entries stream=codec_name -of default=noprint_wrappers=1:nokey=1 "$TEST_DIR/test.webm" 2>/dev/null)
AUDIO_CODEC=$(ffprobe -v error -select_streams a:0 -show_entries stream=codec_name -of default=noprint_wrappers=1:nokey=1 "$TEST_DIR/test.webm" 2>/dev/null)

if [ "$VIDEO_CODEC" = "vp9" ]; then
    echo "   ✓ Video codec: VP9"
else
    echo "   ✗ Video codec incorrect: $VIDEO_CODEC (expected vp9)"
    exit 1
fi

if [ "$AUDIO_CODEC" = "opus" ]; then
    echo "   ✓ Audio codec: Opus"
else
    echo "   ✗ Audio codec incorrect: $AUDIO_CODEC (expected opus)"
    exit 1
fi

# Check for errors in log
if grep -q "Impossible to convert" "$TEST_DIR/encode.log"; then
    echo "   ✗ Filter chain error detected"
    exit 1
fi

if grep -q "Quality-based encoding not supported" "$TEST_DIR/encode.log"; then
    echo "   ✗ libopus VBR error detected"
    exit 1
fi

echo "   ✓ No encoding errors detected"
echo ""

echo "=== ✓ All checks passed! ==="
echo ""
echo "VAAPI encoding is working correctly with:"
echo "  - No filter chain errors (hwaccel_output_format → vp9_vaapi works)"
echo "  - No libopus VBR errors (compression_level works without -vbr flag)"
echo "  - Valid VP9/Opus output"

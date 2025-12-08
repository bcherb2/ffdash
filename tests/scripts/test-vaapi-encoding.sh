#!/bin/bash
# Integration test for VAAPI hardware encoding
# Tests the complete encoding pipeline with synthetic test videos

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test configuration
TEST_DIR="./test_videos"
BINARY="./target/release/ffdash"
TEST_COUNT=0
PASS_COUNT=0
FAIL_COUNT=0

# Cleanup function
cleanup() {
    echo -e "\n${YELLOW}Cleaning up test files...${NC}"
    rm -rf "$TEST_DIR"
}

trap cleanup EXIT

# Print test result
pass() {
    echo -e "${GREEN}✓ PASS${NC}: $1"
    ((PASS_COUNT++))
    ((TEST_COUNT++))
}

fail() {
    echo -e "${RED}✗ FAIL${NC}: $1"
    echo -e "  ${RED}Error: $2${NC}"
    ((FAIL_COUNT++))
    ((TEST_COUNT++))
}

# Check prerequisites
echo -e "${YELLOW}=== VAAPI Encoding Integration Tests ===${NC}\n"

echo "Checking prerequisites..."

# Check if binary exists
if [ ! -f "$BINARY" ]; then
    echo -e "${RED}Error: Binary not found at $BINARY${NC}"
    echo "Run: cargo build --release"
    exit 1
fi

# Check if FFmpeg is available
if ! command -v ffmpeg &> /dev/null; then
    echo -e "${RED}Error: ffmpeg not found in PATH${NC}"
    exit 1
fi

# Check if VAAPI is available
if ! vainfo &> /dev/null; then
    echo -e "${YELLOW}Warning: vainfo not available, skipping VAAPI check${NC}"
else
    echo "VAAPI device info:"
    vainfo 2>&1 | grep -E "(Driver version|VAProfile)" | head -5
fi

echo ""

# Create test directory
mkdir -p "$TEST_DIR"
echo "Created test directory: $TEST_DIR"
echo ""

# Test 1: Generate synthetic test video
echo -e "${YELLOW}[Test 1]${NC} Generating synthetic test video..."
TEST_VIDEO="$TEST_DIR/test_video.mp4"
if ffmpeg -f lavfi -i testsrc=duration=5:size=1920x1080:rate=24 \
    -f lavfi -i sine=frequency=1000:duration=5 \
    -c:v libx264 -pix_fmt yuv420p -c:a aac \
    "$TEST_VIDEO" -y &> /dev/null; then
    pass "Generated test video (5s, 1920x1080, 24fps)"
else
    fail "Failed to generate test video" "FFmpeg error"
    exit 1
fi

# Test 2: Verify test video is valid
echo -e "${YELLOW}[Test 2]${NC} Verifying test video is valid..."
if ffprobe "$TEST_VIDEO" &> /dev/null; then
    DURATION=$(ffprobe -v error -show_entries format=duration -of default=noprint_wrappers=1:nokey=1 "$TEST_VIDEO")
    pass "Test video is valid (duration: ${DURATION}s)"
else
    fail "Test video is not valid" "ffprobe failed"
    exit 1
fi

# Test 3: Initialize ffdash config
echo -e "${YELLOW}[Test 3]${NC} Initializing ffdash configuration..."
CONFIG_FILE="$TEST_DIR/ffdash_config.toml"
if "$BINARY" init-config "$CONFIG_FILE" &> /dev/null; then
    pass "Created ffdash config at $CONFIG_FILE"
else
    fail "Failed to create config" "init-config command failed"
    exit 1
fi

# Test 4: Scan directory for videos
echo -e "${YELLOW}[Test 4]${NC} Scanning test directory..."
if "$BINARY" scan "$TEST_DIR" --config "$CONFIG_FILE" &> /dev/null; then
    pass "Scanned directory successfully"
else
    fail "Directory scan failed" "scan command failed"
    exit 1
fi

# Test 5: Check if encoding state was created
echo -e "${YELLOW}[Test 5]${NC} Verifying encoding state..."
ENC_STATE="$TEST_DIR/.enc_state"
if [ -f "$ENC_STATE" ]; then
    JOB_COUNT=$(grep -c "id =" "$ENC_STATE" || echo "0")
    pass "Encoding state created with $JOB_COUNT job(s)"
else
    fail "Encoding state not created" ".enc_state file missing"
    exit 1
fi

# Test 6: Run actual VAAPI encode (single job)
echo -e "${YELLOW}[Test 6]${NC} Running VAAPI hardware encode..."
OUTPUT_FILE="$TEST_DIR/test_video.webm"

# Create a temporary log file for this encode
ENCODE_LOG="$TEST_DIR/encode_test.log"

# Run encode directly with ffdash (simulating what the worker does)
# We'll use a Rust test program or just verify the command works
if timeout 30 ffmpeg \
    -init_hw_device vaapi=va:/dev/dri/renderD128 \
    -hwaccel vaapi \
    -hwaccel_output_format vaapi \
    -i "$TEST_VIDEO" \
    -progress - -nostats \
    -c:v vp9_vaapi \
    -low_power 1 \
    -rc_mode CQP \
    -global_quality 31 \
    -g 240 \
    -c:a libopus \
    -b:a 128k \
    -compression_level 10 \
    "$OUTPUT_FILE" -y &> "$ENCODE_LOG"; then

    pass "VAAPI encode completed successfully"
else
    EXIT_CODE=$?
    fail "VAAPI encode failed (exit code: $EXIT_CODE)" "See $ENCODE_LOG for details"
    echo -e "\n${RED}Last 20 lines of encode log:${NC}"
    tail -20 "$ENCODE_LOG"
    exit 1
fi

# Test 7: Verify output file exists and has size > 0
echo -e "${YELLOW}[Test 7]${NC} Verifying output file..."
if [ -f "$OUTPUT_FILE" ]; then
    FILE_SIZE=$(stat -f%z "$OUTPUT_FILE" 2>/dev/null || stat -c%s "$OUTPUT_FILE" 2>/dev/null)
    if [ "$FILE_SIZE" -gt 0 ]; then
        SIZE_KB=$((FILE_SIZE / 1024))
        pass "Output file created (${SIZE_KB} KB)"
    else
        fail "Output file is empty" "File size is 0 bytes"
        exit 1
    fi
else
    fail "Output file not created" "$OUTPUT_FILE does not exist"
    exit 1
fi

# Test 8: Verify output is valid WebM/VP9
echo -e "${YELLOW}[Test 8]${NC} Verifying output format..."
if CODEC=$(ffprobe -v error -select_streams v:0 -show_entries stream=codec_name -of default=noprint_wrappers=1:nokey=1 "$OUTPUT_FILE" 2>/dev/null); then
    if [ "$CODEC" = "vp9" ]; then
        pass "Output is valid VP9 video (codec: $CODEC)"
    else
        fail "Output codec is not VP9" "Got: $CODEC, expected: vp9"
    fi
else
    fail "Cannot probe output file" "ffprobe failed"
fi

# Test 9: Verify audio codec
echo -e "${YELLOW}[Test 9]${NC} Verifying audio codec..."
if AUDIO_CODEC=$(ffprobe -v error -select_streams a:0 -show_entries stream=codec_name -of default=noprint_wrappers=1:nokey=1 "$OUTPUT_FILE" 2>/dev/null); then
    if [ "$AUDIO_CODEC" = "opus" ]; then
        pass "Audio codec is Opus (codec: $AUDIO_CODEC)"
    else
        fail "Audio codec is not Opus" "Got: $AUDIO_CODEC, expected: opus"
    fi
else
    fail "Cannot probe audio stream" "ffprobe failed"
fi

# Test 10: Verify output duration matches input
echo -e "${YELLOW}[Test 10]${NC} Verifying output duration..."
INPUT_DUR=$(ffprobe -v error -show_entries format=duration -of default=noprint_wrappers=1:nokey=1 "$TEST_VIDEO" 2>/dev/null)
OUTPUT_DUR=$(ffprobe -v error -show_entries format=duration -of default=noprint_wrappers=1:nokey=1 "$OUTPUT_FILE" 2>/dev/null)

# Allow 0.1s tolerance
if awk -v in="$INPUT_DUR" -v out="$OUTPUT_DUR" 'BEGIN {exit !(out >= in - 0.1 && out <= in + 0.1)}'; then
    pass "Duration matches (input: ${INPUT_DUR}s, output: ${OUTPUT_DUR}s)"
else
    fail "Duration mismatch" "Input: ${INPUT_DUR}s, Output: ${OUTPUT_DUR}s"
fi

# Test 11: Check encode log for VAAPI usage confirmation
echo -e "${YELLOW}[Test 11]${NC} Verifying VAAPI was actually used..."
if grep -q "vp9_vaapi" "$ENCODE_LOG" && ! grep -q "libvpx-vp9" "$ENCODE_LOG"; then
    pass "VAAPI encoder was used (not software fallback)"
else
    fail "VAAPI may not have been used" "Check $ENCODE_LOG"
fi

# Test 12: Verify no filter chain errors
echo -e "${YELLOW}[Test 12]${NC} Checking for filter chain errors..."
if grep -q "Impossible to convert between the formats" "$ENCODE_LOG"; then
    fail "Filter chain error detected" "VAAPI format conversion failed"
elif grep -q "Quality-based encoding not supported" "$ENCODE_LOG"; then
    fail "libopus VBR error detected" "Invalid audio parameters"
else
    pass "No filter chain or audio encoding errors"
fi

# Summary
echo -e "\n${YELLOW}=== Test Summary ===${NC}"
echo -e "Total tests: $TEST_COUNT"
echo -e "${GREEN}Passed: $PASS_COUNT${NC}"
echo -e "${RED}Failed: $FAIL_COUNT${NC}"

if [ $FAIL_COUNT -eq 0 ]; then
    echo -e "\n${GREEN}✓ All tests passed!${NC}"
    exit 0
else
    echo -e "\n${RED}✗ Some tests failed${NC}"
    exit 1
fi

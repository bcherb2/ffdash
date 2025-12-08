#!/bin/bash
# Remote VAAPI Testing Script
#
# This script syncs code to the remote Docker container and runs the
# parameter coverage tests. The remote container has VAAPI hardware
# support for full integration testing.
#
# Usage:
#   ./tests/run_vaapi_tests.sh [test_filter]
#
# Examples:
#   ./tests/run_vaapi_tests.sh                    # Run all tests
#   ./tests/run_vaapi_tests.sh parameter_coverage # Run only parameter coverage tests
#   ./tests/run_vaapi_tests.sh parallelism        # Run only parallelism tests

set -e  # Exit on error

# Configuration
REMOTE_HOST="root@192.168.1.99"
REMOTE_PORT="2224"
REMOTE_PATH="/build"
TEST_FILTER="${1:-}"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== VAAPI Parameter Coverage Test Runner ===${NC}\n"

# Step 1: Check local git status
echo -e "${YELLOW}[1/5]${NC} Checking local git status..."
if ! git diff-index --quiet HEAD --; then
    echo -e "${YELLOW}Warning: You have uncommitted changes${NC}"
    git status --short
    echo ""
fi

# Get current commit
CURRENT_COMMIT=$(git log -1 --oneline)
echo "Current commit: $CURRENT_COMMIT"
echo ""

# Step 2: Sync code to remote container
echo -e "${YELLOW}[2/5]${NC} Syncing code to remote container..."
echo "Creating tarball (excluding target/ and .git/)..."

if tar czf - --exclude target --exclude .git . | \
   ssh -p "$REMOTE_PORT" "$REMOTE_HOST" "cd $REMOTE_PATH && tar xzf -"; then
    echo -e "${GREEN}✓ Code synced successfully${NC}"
else
    echo -e "${RED}✗ Failed to sync code${NC}"
    exit 1
fi
echo ""

# Step 3: Verify sync
echo -e "${YELLOW}[3/5]${NC} Verifying sync on remote..."
REMOTE_COMMIT=$(ssh -p "$REMOTE_PORT" "$REMOTE_HOST" "cd $REMOTE_PATH && git log -1 --oneline 2>/dev/null || echo 'unknown'")
echo "Remote commit: $REMOTE_COMMIT"

if [ "$CURRENT_COMMIT" != "$REMOTE_COMMIT" ]; then
    echo -e "${YELLOW}Warning: Local and remote commits differ${NC}"
fi
echo ""

# Step 4: Build on remote
echo -e "${YELLOW}[4/5]${NC} Building on remote container..."
if ssh -p "$REMOTE_PORT" "$REMOTE_HOST" "cd $REMOTE_PATH && export PATH=/root/.cargo/bin:\$PATH && cargo build --tests 2>&1"; then
    echo -e "${GREEN}✓ Build successful${NC}"
else
    echo -e "${RED}✗ Build failed${NC}"
    exit 1
fi
echo ""

# Step 5: Run tests
echo -e "${YELLOW}[5/5]${NC} Running parameter coverage tests..."
echo ""

if [ -z "$TEST_FILTER" ]; then
    echo "Running ALL parameter coverage tests..."
    TEST_CMD="export PATH=/root/.cargo/bin:\$PATH && cargo test --test integration -- parameter_coverage vaapi_vs_software_parity --nocapture --test-threads=1"
else
    echo "Running tests matching: $TEST_FILTER"
    TEST_CMD="export PATH=/root/.cargo/bin:\$PATH && cargo test --test integration -- $TEST_FILTER --nocapture --test-threads=1"
fi

if ssh -p "$REMOTE_PORT" "$REMOTE_HOST" "cd $REMOTE_PATH && $TEST_CMD"; then
    echo ""
    echo -e "${GREEN}✓ All tests passed!${NC}"
    echo ""
    echo "The parameter coverage test harness is working correctly."
    echo "It would have caught the parallelism bug (commit 0b80e0b)."
else
    echo ""
    echo -e "${RED}✗ Tests failed${NC}"
    echo ""
    echo "If tests are failing due to missing parameters in VAAPI commands,"
    echo "this means the test harness caught a bug! Check the error messages"
    echo "to see which parameters are missing and add them to build_vaapi_cmd()"
    echo "in src/engine/mod.rs."
    exit 1
fi

echo ""
echo -e "${GREEN}=== Test run complete ===${NC}"

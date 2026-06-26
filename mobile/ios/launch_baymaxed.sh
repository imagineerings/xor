#!/bin/bash
set -e

# Configuration
PORT=62996
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BAYMAXD_URL="https://github.com/michaelneale/baymax-tunnel/releases/download/test/baymaxd"
BAYMAXD_LOCAL_PATH="${SCRIPT_DIR}/baymaxd"

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

SECRET="test"

# Function to download baymaxd binary with LOUD notification
download_baymaxd() {
    echo ""
    echo -e "${BOLD}${MAGENTA}╔═══════════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BOLD}${MAGENTA}║                                                                       ║${NC}"
    echo -e "${BOLD}${MAGENTA}║                   🚀  DOWNLOADING BAYMAXD BINARY  🚀                   ║${NC}"
    echo -e "${BOLD}${MAGENTA}║                                                                       ║${NC}"
    echo -e "${BOLD}${MAGENTA}╚═══════════════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "${BOLD}${CYAN}⬇️  Fetching from: ${BAYMAXD_URL}${NC}"
    echo -e "${BOLD}${CYAN}📦 Saving to: ${BAYMAXD_LOCAL_PATH}${NC}"
    echo ""
    
    if curl -L -o "$BAYMAXD_LOCAL_PATH" "$BAYMAXD_URL"; then
        chmod +x "$BAYMAXD_LOCAL_PATH"
        echo ""
        echo -e "${BOLD}${GREEN}╔═══════════════════════════════════════════════════════════════════════╗${NC}"
        echo -e "${BOLD}${GREEN}║                                                                       ║${NC}"
        echo -e "${BOLD}${GREEN}║                  ✅  DOWNLOAD SUCCESSFUL!  ✅                         ║${NC}"
        echo -e "${BOLD}${GREEN}║                                                                       ║${NC}"
        echo -e "${BOLD}${GREEN}║              baymaxd binary is now available locally!                  ║${NC}"
        echo -e "${BOLD}${GREEN}║                                                                       ║${NC}"
        echo -e "${BOLD}${GREEN}╚═══════════════════════════════════════════════════════════════════════╝${NC}"
        echo ""
        sleep 1  # Pause so user can see the message
        return 0
    else
        echo ""
        echo -e "${BOLD}${RED}╔═══════════════════════════════════════════════════════════════════════╗${NC}"
        echo -e "${BOLD}${RED}║                                                                       ║${NC}"
        echo -e "${BOLD}${RED}║                    ❌  DOWNLOAD FAILED!  ❌                           ║${NC}"
        echo -e "${BOLD}${RED}║                                                                       ║${NC}"
        echo -e "${BOLD}${RED}╚═══════════════════════════════════════════════════════════════════════╝${NC}"
        echo ""
        return 1
    fi
}

# Determine which baymaxd to use
BAYMAXD_CMD=""
if command -v baymaxd &> /dev/null; then
    # Found in PATH
    BAYMAXD_CMD="baymaxd"
    echo ""
    echo -e "${BOLD}${BLUE}╔═══════════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BOLD}${BLUE}║                                                                       ║${NC}"
    echo -e "${BOLD}${BLUE}║                   📍  USING BAYMAXD FROM PATH  📍                      ║${NC}"
    echo -e "${BOLD}${BLUE}║                                                                       ║${NC}"
    echo -e "${BOLD}${BLUE}╚═══════════════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "${BOLD}${CYAN}📂 Location: $(which baymaxd)${NC}"
    echo ""
elif [ -f "$BAYMAXD_LOCAL_PATH" ]; then
    # Found locally
    BAYMAXD_CMD="$BAYMAXD_LOCAL_PATH"
    echo ""
    echo -e "${BOLD}${YELLOW}╔═══════════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BOLD}${YELLOW}║                                                                       ║${NC}"
    echo -e "${BOLD}${YELLOW}║                  📦  USING LOCAL BAYMAXD BINARY  📦                    ║${NC}"
    echo -e "${BOLD}${YELLOW}║                                                                       ║${NC}"
    echo -e "${BOLD}${YELLOW}╚═══════════════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "${BOLD}${CYAN}📂 Location: $BAYMAXD_LOCAL_PATH${NC}"
    echo ""
else
    # Not found anywhere - download it
    if download_baymaxd; then
        BAYMAXD_CMD="$BAYMAXD_LOCAL_PATH"
    else
        echo -e "${RED}Error: Failed to download baymaxd${NC}"
        echo -e "${YELLOW}Please manually download from: ${BAYMAXD_URL}${NC}"
        echo -e "${YELLOW}Or add baymax/target/release to your PATH${NC}"
        exit 1
    fi
fi

# Cleanup function
cleanup() {
    echo -e "\n${YELLOW}Shutting down...${NC}"
    if [ ! -z "$BAYMAXD_PID" ]; then
        echo "Stopping baymaxd (PID: $BAYMAXD_PID)"
        kill $BAYMAXD_PID 2>/dev/null || true
    fi
    exit 0
}

trap cleanup SIGINT SIGTERM EXIT

# Start baymaxd in the background
echo -e "${GREEN}Starting baymaxd on port ${PORT}...${NC}"
export BAYMAX_PORT=$PORT
export BAYMAX_SERVER__SECRET_KEY="$SECRET"
$BAYMAXD_CMD agent &
BAYMAXD_PID=$!

# Wait for baymaxd to be ready
echo "Waiting for baymaxd to start..."
for i in {1..30}; do
    if curl -s "http://localhost:${PORT}/health" > /dev/null 2>&1; then
        echo -e "${GREEN}✓ Baymaxd is running (PID: $BAYMAXD_PID)${NC}"
        break
    fi
    if [ $i -eq 30 ]; then
        echo -e "${RED}Error: baymaxd failed to start${NC}"
        exit 1
    fi
    sleep 0.5
done

echo ""
echo -e "${BLUE}╔════════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                     Connection Information                         ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${GREEN}URL:${NC}        http://localhost:${PORT}"
echo -e "${GREEN}Secret Key:${NC} $SECRET"
echo ""
echo -e "${GREEN}✓ Baymaxd is running!${NC}"
echo -e "${YELLOW}Press Ctrl+C to stop the server${NC}"
echo ""

# Keep the script running
wait

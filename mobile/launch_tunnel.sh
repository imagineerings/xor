#!/bin/bash
set -e

# =============================================================================
# Baymax Tunnel Launcher
#
# Builds and runs the baymaxed-tunnel (Tailscale tunnel) from the local Go
# source at mobile/baymax-tunnel/, or falls back to a mock server for testing.
#
# Supports both iOS (QR codes, deep links) and Android (emulator 10.0.2.2
# mapping, adb deep link push) workflows.
#
# Usage:
#   ./mobile/launch_tunnel.sh [--mock] [--port PORT] [--secret SECRET]
#
#   --mock       Run a lightweight Python mock server instead of baymaxed
#                (useful when the baymaxed Rust binary isn't available).
#   --port PORT  Local port (default: 62996).
#   --secret KEY Secret key (default: auto-generated).
# =============================================================================

# --- Configuration -----------------------------------------------------------
PORT=${PORT:-62996}
SECRET="${SECRET:-test}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TUNNEL_SRC="$PROJECT_ROOT/mobile/baymax-tunnel"
TUNNEL_BIN="$TUNNEL_SRC/baymaxed-tunnel"
MODE="auto"   # auto | mock

# --- Colors ------------------------------------------------------------------
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

# -----------------------------------------------------------------------------
# Parse arguments
# -----------------------------------------------------------------------------
while [[ $# -gt 0 ]]; do
    case "$1" in
        --mock) MODE="mock"; shift ;;
        --port) PORT="$2"; shift 2 ;;
        --secret) SECRET="$2"; shift 2 ;;
        -h|--help)
            echo "Baymax Tunnel Launcher"
            echo ""
            echo "  --mock       Run mock server (no baymaxed binary needed)"
            echo "  --port PORT  Local port (default: 62996)"
            echo "  --secret KEY Secret key (default: auto-generated)"
            exit 0
            ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

# -----------------------------------------------------------------------------
# Build baymaxed-tunnel from local Go source
# -----------------------------------------------------------------------------
build_tunnel() {
    if [ ! -d "$TUNNEL_SRC" ]; then
        echo -e "${RED}Error: Tunnel source not found at $TUNNEL_SRC${NC}"
        exit 1
    fi

    # Skip build if binary already exists
    if [ -x "$TUNNEL_BIN" ]; then
        echo -e "${GREEN}✓ Tunnel binary already built: ${TUNNEL_BIN}${NC}"
        return 0
    fi

    if ! command -v go &>/dev/null; then
        echo -e "${RED}Error: Go is required to build the tunnel binary.${NC}"
        echo -e "${YELLOW}Install Go from https://go.dev/dl/${NC}"
        exit 1
    fi

    echo ""
    echo -e "${BOLD}${MAGENTA}╔═══════════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BOLD}${MAGENTA}║               🔨  BUILDING BAYMAXED-TUNNEL FROM SOURCE              ║${NC}"
    echo -e "${BOLD}${MAGENTA}╚═══════════════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "${CYAN}📂 Source: ${TUNNEL_SRC}${NC}"
    echo ""

    pushd "$TUNNEL_SRC" > /dev/null
    if go build -o "$TUNNEL_BIN" . 2>&1; then
        echo -e "${GREEN}✓ Built successfully: ${TUNNEL_BIN}${NC}"
    else
        echo -e "${RED}✗ Build failed${NC}"
        echo -e "${YELLOW}Check Go installation and dependencies in ${TUNNEL_SRC}${NC}"
        popd > /dev/null
        exit 1
    fi
    popd > /dev/null

    echo ""
}

# -----------------------------------------------------------------------------
# Start a lightweight Python mock server for Android testing
# -----------------------------------------------------------------------------
MOCK_SERVER_PID=""
start_mock_server() {
    echo ""
    echo -e "${BOLD}${YELLOW}╔═══════════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BOLD}${YELLOW}║               🧪  STARTING MOCK SERVER (TEST MODE)                   ║${NC}"
    echo -e "${BOLD}${YELLOW}║                                                                       ║${NC}"
    echo -e "${BOLD}${YELLOW}║   No baymaxed binary found — using a lightweight mock server.         ║${NC}"
    echo -e "${BOLD}${YELLOW}║   This is suitable for UI/API integration testing only.               ║${NC}"
    echo -e "${BOLD}${YELLOW}╚═══════════════════════════════════════════════════════════════════════╝${NC}"
    echo ""

    local port="$1"
    local secret="$2"

    # Prefer Python 3, fall back to Python
    PYTHON=""
    if command -v python3 &>/dev/null; then
        PYTHON="python3"
    elif command -v python &>/dev/null; then
        PYTHON="python"
    else
        echo -e "${RED}Error: Python 3 is required for mock server mode.${NC}"
        echo -e "${YELLOW}Install Python from https://www.python.org/downloads/${NC}"
        exit 1
    fi

    echo -e "${CYAN}🐍 Using Python: $(which $PYTHON)${NC}"
    echo -e "${CYAN}🔌 Port: ${port}${NC}"
    echo -e "${CYAN}🔑 Secret: ${secret}${NC}"
    echo ""

    # Launch a Python HTTP server embedded as a script that responds to the
    # Android app's API endpoints. We write it to a temp file to avoid shell
    # variable expansion issues with the heredoc.
    # Create a temp file for the Python mock server script.
    # macOS mktemp requires exactly 6 X's; Linux is flexible.
    MOCK_SCRIPT="$(mktemp 2>/dev/null || mktemp -t baymax-mock-server)"
    # Ensure .py extension (for editor association); macOS mktemp gives a bare name
    if [[ "$MOCK_SCRIPT" != *.py ]]; then
        mv "$MOCK_SCRIPT" "${MOCK_SCRIPT}.py" 2>/dev/null || true
        MOCK_SCRIPT="${MOCK_SCRIPT}.py"
    fi
    cat > "$MOCK_SCRIPT" << 'PYEOF'
import http.server
import json
import os
import sys
import urllib.parse

PORT = int(os.environ.get('MOCK_PORT', '62996'))
SECRET = os.environ.get('MOCK_SECRET', 'test')

class MockHandler(http.server.BaseHTTPRequestHandler):
    def _json(self, data, status=200):
        self.send_response(status)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Access-Control-Allow-Origin', '*')
        self.end_headers()
        self.wfile.write(json.dumps(data).encode())

    def _sse(self):
        self.send_response(200)
        self.send_header('Content-Type', 'text/event-stream')
        self.send_header('Cache-Control', 'no-cache')
        self.send_header('Access-Control-Allow-Origin', '*')
        self.end_headers()

    def do_GET(self):
        parsed = urllib.parse.urlparse(self.path)
        path = parsed.path.rstrip('/')

        if path == '/status':
            self._json({'status': 'ok', 'version': 'mock-1.0.0'})
        elif path == '/health':
            self._json({'status': 'healthy'})
        elif path == '/sessions':
            self._json({
                'sessions': [
                    {'id': 'mock-001', 'description': 'Mock Session 1', 'message_count': 12, 'created_at': '2026-06-28T10:00:00Z', 'updated_at': '2026-06-28T12:30:00Z'},
                    {'id': 'mock-002', 'description': 'Mock Session 2', 'message_count': 5, 'created_at': '2026-06-28T14:00:00Z', 'updated_at': '2026-06-28T15:00:00Z'},
                ]
            })
        elif path == '/sessions/insights':
            self._json({'total_sessions': 2, 'total_tokens': 150000})
        elif path == '/config/extensions':
            self._json([])
        elif path.startswith('/sessions/') and '/messages' in path:
            self._json({'messages': []})
        elif path.startswith('/sessions/'):
            session_id = path.split('/')[-1]
            self._json({'id': session_id, 'description': 'Mock Session', 'message_count': 7, 'created_at': '2026-06-28T10:00:00Z', 'updated_at': '2026-06-28T12:30:00Z'})
        else:
            self._json({'error': 'not_found'}, 404)

    def do_POST(self):
        parsed = urllib.parse.urlparse(self.path)
        path = parsed.path.rstrip('/')
        content_len = int(self.headers.get('Content-Length', 0))
        body = self.rfile.read(content_len).decode() if content_len else '{}'

        if path == '/agent/start':
            import uuid
            session_id = str(uuid.uuid4())[:8]
            self._json({'id': f'mock-{session_id}', 'status': 'started'})
        elif path == '/agent/resume':
            self._json({'id': 'mock-001', 'status': 'resumed'})
        elif path == '/agent/update_provider':
            self._json({'status': 'ok'})
        elif path == '/agent/update_from_session':
            self._json({'status': 'ok'})
        elif path == '/reply':
            self._sse()
            import time as _time
            for ev in [{'type': 'text', 'content': 'Hello! I am the mock assistant.'}, {'type': 'finish', 'reason': 'stop'}]:
                self.wfile.write(f'data: {json.dumps(ev)}\n\n'.encode())
                self.wfile.flush()
                _time.sleep(0.1)
        else:
            self._json({'error': 'not_found'}, 404)

    def log_message(self, format, *args):
        print(f'[mock] {args[0]} {args[1]} {args[2]}', flush=True)

server = http.server.HTTPServer(('0.0.0.0', PORT), MockHandler)
print(f'[mock] Mock Baymax server running on port {PORT}')
sys.stdout.flush()
server.serve_forever()
PYEOF

    MOCK_PORT="$port" MOCK_SECRET="$secret" $PYTHON "$MOCK_SCRIPT" &
    MOCK_SERVER_PID=$!
    echo -e "${GREEN}✓ Mock server started (PID: $MOCK_SERVER_PID)${NC}"

    # Wait for mock server to be ready
    echo "Waiting for mock server to start..."
    for i in {1..15}; do
        if curl -s "http://localhost:${port}/status" > /dev/null 2>&1; then
            echo -e "${GREEN}✓ Mock server is ready${NC}"
            return 0
        fi
        sleep 0.5
    done

    echo -e "${RED}Error: Mock server failed to start${NC}"
    return 1
}

# -----------------------------------------------------------------------------
# Cleanup
# -----------------------------------------------------------------------------
cleanup() {
    echo -e "\n${YELLOW}Shutting down...${NC}"
    if [ ! -z "$TUNNEL_PID" ]; then
        echo "Stopping tunnel (PID: $TUNNEL_PID)"
        kill $TUNNEL_PID 2>/dev/null || true
    fi
    if [ ! -z "$MOCK_SERVER_PID" ]; then
        echo "Stopping mock server (PID: $MOCK_SERVER_PID)"
        kill $MOCK_SERVER_PID 2>/dev/null || true
    fi
    if [ ! -z "$BAYMAXED_PID" ]; then
        echo "Stopping baymaxed (PID: $BAYMAXED_PID)"
        kill $BAYMAXED_PID 2>/dev/null || true
    fi
    exit 0
}

trap cleanup SIGINT SIGTERM EXIT

# =============================================================================
# Main
# =============================================================================

echo ""
echo -e "${BOLD}${BLUE}╔════════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BOLD}${BLUE}║               🚀  Baymax Tunnel Launcher  🚀                       ║${NC}"
echo -e "${BOLD}${BLUE}║                                                                     ║${NC}"
echo -e "${BOLD}${BLUE}║  Local source: mobile/baymax-tunnel/                                ║${NC}"
echo -e "${BOLD}${BLUE}╚════════════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Build the tunnel binary
build_tunnel

# Determine mode
FOUND_BAYMAXED=false
if command -v baymaxed &>/dev/null; then
    FOUND_BAYMAXED=true
    echo -e "${GREEN}✓ baymaxed found in PATH at $(which baymaxed)${NC}"
elif [ -f "$PROJECT_ROOT/target/release/baymaxed" ]; then
    FOUND_BAYMAXED=true
    export PATH="$PROJECT_ROOT/target/release:$PATH"
    echo -e "${GREEN}✓ baymaxed found at $PROJECT_ROOT/target/release/baymaxed${NC}"
elif [ -f "$PROJECT_ROOT/target/debug/baymaxed" ]; then
    FOUND_BAYMAXED=true
    export PATH="$PROJECT_ROOT/target/debug:$PATH"
    echo -e "${GREEN}✓ baymaxed found at $PROJECT_ROOT/target/debug/baymaxed${NC}"
fi

if [ "$MODE" = "mock" ] || [ "$FOUND_BAYMAXED" = false ]; then
    # --- Mock server mode ----------------------------------------------------
    if [ "$MODE" = "mock" ]; then
        echo -e "${YELLOW}➡️  Mock mode requested via --mock flag${NC}"
    else
        echo -e "${YELLOW}⚠️  baymaxed binary not found — falling back to mock server${NC}"
        echo -e "${YELLOW}   For production use, build baymaxed from Rust source:${NC}"
        echo -e "${YELLOW}   cargo build --release -p baymax${NC}"
    fi
    echo ""

    if ! start_mock_server "$PORT" "$SECRET"; then
        echo -e "${RED}Failed to start mock server${NC}"
        exit 1
    fi

    SERVER_PID=$MOCK_SERVER_PID
    SERVER_URL="http://localhost:${PORT}"
    SERVER_TYPE="Mock"
else
    # --- Production mode: run baymaxed ---------------------------------------
    echo ""
    echo -e "${BOLD}${GREEN}╔═══════════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BOLD}${GREEN}║               🚀  STARTING BAYMAXED AGENT DAEMON                      ║${NC}"
    echo -e "${BOLD}${GREEN}╚═══════════════════════════════════════════════════════════════════════╝${NC}"
    echo ""

    echo -e "${GREEN}Starting baymaxed on port ${PORT}...${NC}"
    export BAYMAX_PORT=$PORT
    export BAYMAX_SERVER__SECRET_KEY="$SECRET"
    baymaxed agent &
    BAYMAXED_PID=$!

    # Wait for baymaxed to be ready
    echo "Waiting for baymaxed to start..."
    for i in {1..30}; do
        if curl -s "http://localhost:${PORT}/health" > /dev/null 2>&1; then
            echo -e "${GREEN}✓ Baymaxed is running (PID: $BAYMAXED_PID)${NC}"
            break
        fi
        if [ $i -eq 30 ]; then
            echo -e "${RED}Error: baymaxed failed to start${NC}"
            exit 1
        fi
        sleep 0.5
    done

    SERVER_PID=$BAYMAXED_PID
    SERVER_URL="http://localhost:${PORT}"
    SERVER_TYPE="Baymaxed"
fi

echo ""
echo -e "${BLUE}╔════════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                     Connection Information                         ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${GREEN}Server:${NC}       ${SERVER_TYPE}"
echo -e "${GREEN}URL:${NC}          ${SERVER_URL}"
echo -e "${GREEN}Secret Key:${NC}   ${SECRET}"
echo ""

# -----------------------------------------------------------------------------
# Android-specific instructions
# -----------------------------------------------------------------------------
echo -e "${BOLD}${CYAN}╔════════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BOLD}${CYAN}║                    Android Emulator Quick Start                     ║${NC}"
echo -e "${BOLD}${CYAN}╚════════════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${YELLOW}Emulator URL:${NC} http://10.0.2.2:${PORT}"
echo ""
echo -e "${YELLOW}To configure the Android app, run this adb command:${NC}"
echo ""
echo -e "  ${GREEN}adb shell am start \\\\${NC}"
echo -e "  ${GREEN}  -a android.intent.action.VIEW \\\\${NC}"
echo -e "  ${GREEN}  -d \"baymaxchat://configure?data=\$(python3 -c 'import urllib.parse, json; print(urllib.parse.quote(json.dumps({\"url\": \"http://10.0.2.2:${PORT}\", \"secret\": \"${SECRET}\"})))')\"${NC}"
echo ""
echo -e "${YELLOW}Or with the app already running:${NC}"
echo ""
CONFIG_JSON="{\"url\":\"http://10.0.2.2:${PORT}\",\"secret\":\"${SECRET}\"}"
URL_ENCODED=$(python3 -c "import urllib.parse, json; print(urllib.parse.quote(json.dumps({'url': 'http://10.0.2.2:${PORT}', 'secret': '${SECRET}'})))" 2>/dev/null || \
              python -c "import urllib.parse, json; print(urllib.parse.quote(json.dumps({'url': 'http://10.0.2.2:${PORT}', 'secret': '${SECRET}'})))" 2>/dev/null || \
              echo "URL-ENCODING-UNAVAILABLE")
DEEP_LINK="baymaxchat://configure?data=${URL_ENCODED}"
echo -e "  ${GREEN}adb shell am start -a android.intent.action.VIEW -d \"${DEEP_LINK}\"${NC}"
echo ""
echo -e "${YELLOW}To verify:${NC}"
echo -e "  ${CYAN}adb logcat | grep -E \"(Configuration|✅|BaymaxApi|MockServer)\"${NC}"
echo ""

# -----------------------------------------------------------------------------
# iOS-specific: QR code
# -----------------------------------------------------------------------------
# Build a deep link for iOS / QR code scanning
if command -v qrencode &>/dev/null; then
    IOS_CONFIG_JSON="{\"url\":\"http://localhost:${PORT}\",\"secret\":\"${SECRET}\"}"
    IOS_URL_ENCODED=$(python3 -c "import urllib.parse, json; print(urllib.parse.quote(json.dumps({'url': 'http://localhost:${PORT}', 'secret': '${SECRET}'})))" 2>/dev/null || \
                      python -c "import urllib.parse, json; print(urllib.parse.quote(json.dumps({'url': 'http://localhost:${PORT}', 'secret': '${SECRET}'})))" 2>/dev/null || \
                      echo "")
    IOS_DEEP_LINK="goosechat://configure?data=${IOS_URL_ENCODED}"

    echo -e "${BOLD}${MAGENTA}╔════════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BOLD}${MAGENTA}║                          QR Code (Scan Me!)                        ║${NC}"
    echo -e "${BOLD}${MAGENTA}╚════════════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo "$IOS_DEEP_LINK" | qrencode -t ANSIUTF8 2>/dev/null || echo -e "${YELLOW}(qrencode skipped)${NC}"
    echo ""
fi

# -----------------------------------------------------------------------------
# Keep running
# -----------------------------------------------------------------------------
echo -e "${GREEN}✓ ${SERVER_TYPE} server is running!${NC}"
echo -e "${YELLOW}Press Ctrl+C to stop${NC}"
echo ""

wait

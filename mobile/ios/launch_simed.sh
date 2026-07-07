#!/bin/bash
# =============================================================================
# [DEPRECATED] Use mobile/launch_tunnel.sh instead
# =============================================================================
# This script is kept for backward compatibility. It now delegates to the
# cross-platform launcher at mobile/launch_tunnel.sh.
#
# The old download URL (https://github.com/michaelneale/sim-tunnel/releases/...)
# is stale — the sim-tunnel source now lives at mobile/sim-tunnel/ in this
# repo, and the Go binary is built locally.
#
# New script: ./mobile/launch_tunnel.sh [--mock] [--port PORT] [--secret SECRET]
#
#   --mock       Run a lightweight mock server when simed isn't available.
#   --port PORT  Local port (default: 62996).
#   --secret KEY Secret key (default: test).
# =============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo ""
echo "╔════════════════════════════════════════════════════════════════════╗"
echo "║  [DEPRECATED]  Use mobile/launch_tunnel.sh                       ║"
echo "╚════════════════════════════════════════════════════════════════════╝"
echo ""
echo "  The old download URL is stale. Use the cross-platform launcher:"
echo ""
echo "    ./mobile/launch_tunnel.sh [--mock] [--port PORT] [--secret SECRET]"
echo ""
echo "  This builds simed-tunnel from local Go source at"
echo "  mobile/sim-tunnel/ and supports both iOS and Android."
echo ""

# Auto-redirect: run the new script with the same args
NEW_SCRIPT="$PROJECT_ROOT/mobile/launch_tunnel.sh"
if [ -x "$NEW_SCRIPT" ]; then
    echo "→ Redirecting to: $NEW_SCRIPT $*"
    echo ""
    exec "$NEW_SCRIPT" "$@"
else
    echo "Error: $NEW_SCRIPT not found or not executable."
    echo "Please ensure the mobile/launch_tunnel.sh script exists."
    exit 1
fi

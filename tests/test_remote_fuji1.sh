#!/bin/bash
# Remote Server (fuji1) Integration Tests
#
# Tests SSH tunnel establishment, broker connectivity via tunnel,
# and Claude CLI execution on the remote server.
#
# Prerequisites:
#   - SSH access to dice-fuji1 as hyq
#   - Broker running on fuji1 port 9999
#   - Claude CLI installed on fuji1 at ~/.local/bin/claude
#   - `ws` npm package installed in frontend/
#
# Usage: ./tests/test_remote_fuji1.sh

REMOTE_HOST="${REMOTE_HOST:-dice-fuji1}"
REMOTE_USER="${REMOTE_USER:-hyq}"
REMOTE_BROKER_PORT="${REMOTE_BROKER_PORT:-9999}"
LOCAL_TUNNEL_PORT="${LOCAL_TUNNEL_PORT:-14099}"
REMOTE_CWD="${REMOTE_CWD:-/home/hyq/Projects/CyberGraph}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
export NODE_PATH="$PROJECT_DIR/node_modules:$PROJECT_DIR/frontend/node_modules"

PASS=0
FAIL=0
TUNNEL_PID=""

green() { printf "\033[32m%s\033[0m\n" "$1"; }
red()   { printf "\033[31m%s\033[0m\n" "$1"; }
bold()  { printf "\033[1m%s\033[0m\n" "$1"; }

assert() {
  local name="$1" condition="$2"
  if [ "$condition" = "true" ]; then
    green "  PASS: $name"
    PASS=$((PASS + 1))
  else
    red "  FAIL: $name"
    FAIL=$((FAIL + 1))
  fi
}

cleanup() {
  if [ -n "$TUNNEL_PID" ]; then
    kill "$TUNNEL_PID" 2>/dev/null || true
    wait "$TUNNEL_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

bold "========================================="
bold "  Remote Server (fuji1) Integration Tests"
bold "========================================="
echo ""

# ---- Test 1: SSH Connectivity ----
bold "[1] SSH Connectivity"
if ssh -o ConnectTimeout=5 -o BatchMode=yes "$REMOTE_USER@$REMOTE_HOST" "echo ok" 2>/dev/null | grep -q ok; then
  assert "SSH connection to $REMOTE_HOST" "true"
else
  assert "SSH connection to $REMOTE_HOST" "false"
  red "  Cannot connect to remote host. Aborting."
  exit 1
fi

# ---- Test 2: Remote Broker Check ----
bold "[2] Remote Broker Status"
BROKER_PID=$(ssh "$REMOTE_USER@$REMOTE_HOST" "pgrep -f 'cloudcli-broker.*--port $REMOTE_BROKER_PORT'" 2>/dev/null || echo "")
if [ -n "$BROKER_PID" ]; then
  assert "Broker running on $REMOTE_HOST:$REMOTE_BROKER_PORT" "true"
else
  assert "Broker running on $REMOTE_HOST:$REMOTE_BROKER_PORT" "false"
  red "  Broker not running. Start it with: cloudcli-broker --port $REMOTE_BROKER_PORT"
  exit 1
fi

# ---- Test 3: Remote Claude CLI ----
bold "[3] Remote Claude CLI"
CLAUDE_PATH=$(ssh "$REMOTE_USER@$REMOTE_HOST" "ls ~/.local/bin/claude 2>/dev/null || which claude 2>/dev/null || echo ''")
if [ -n "$CLAUDE_PATH" ]; then
  assert "Claude CLI found at $CLAUDE_PATH" "true"
else
  assert "Claude CLI found on remote" "false"
fi

CLAUDE_VER=$(ssh "$REMOTE_USER@$REMOTE_HOST" "~/.local/bin/claude --version 2>/dev/null || echo ''" | head -1)
if [ -n "$CLAUDE_VER" ]; then
  assert "Claude CLI version: $CLAUDE_VER" "true"
else
  assert "Claude CLI version check" "false"
fi

# ---- Test 4: SSH Tunnel ----
bold "[4] SSH Tunnel Establishment"
# Kill any existing tunnel on this port
kill $(lsof -ti ":$LOCAL_TUNNEL_PORT" 2>/dev/null) 2>/dev/null || true
sleep 1

ssh -f -N -L "$LOCAL_TUNNEL_PORT:127.0.0.1:$REMOTE_BROKER_PORT" \
  -o ServerAliveInterval=30 \
  -o ServerAliveCountMax=3 \
  -o ExitOnForwardFailure=yes \
  -o ConnectTimeout=10 \
  "$REMOTE_USER@$REMOTE_HOST"
sleep 2

TUNNEL_PID=$(lsof -ti ":$LOCAL_TUNNEL_PORT" 2>/dev/null | head -1)
if [ -n "$TUNNEL_PID" ] && lsof -i ":$LOCAL_TUNNEL_PORT" >/dev/null 2>&1; then
  assert "SSH tunnel on localhost:$LOCAL_TUNNEL_PORT" "true"
else
  assert "SSH tunnel on localhost:$LOCAL_TUNNEL_PORT" "false"
  red "  Tunnel failed. Aborting."
  exit 1
fi

# ---- Test 5: Broker Ping via Tunnel ----
bold "[5] Broker Ping via Tunnel"
PING_RESULT=$(cd "$PROJECT_DIR" && timeout 15 node -e "
const WebSocket = require('ws');
const ws = new WebSocket('ws://127.0.0.1:$LOCAL_TUNNEL_PORT');
ws.on('open', () => ws.send(JSON.stringify({type:'ping'})));
ws.on('message', (d) => {
  const m = JSON.parse(d.toString());
  if (m.type === 'pong') {
    console.log(JSON.stringify(m));
    ws.close();
  }
});
ws.on('error', (e) => { console.log('ERROR:' + e.message); ws.close(); });
setTimeout(() => { console.log('TIMEOUT'); process.exit(1); }, 10000);
" 2>&1)

if echo "$PING_RESULT" | grep -q '"type":"pong"'; then
  assert "Broker pong received via tunnel" "true"
  VERSION=$(echo "$PING_RESULT" | python3 -c "import sys,json; print(json.load(sys.stdin)['version'])" 2>/dev/null || echo "unknown")
  echo "    Broker version: $VERSION"
else
  assert "Broker pong received via tunnel" "false"
  echo "    Response: $PING_RESULT"
fi

# ---- Test 6: Claude Command via Tunnel ----
bold "[6] Claude Command via Tunnel"
echo "  Sending: 'What is 2+2? Reply with just the number.'"
CLAUDE_RESULT=$(cd "$PROJECT_DIR" && timeout 120 node -e "
const WebSocket = require('ws');
const ws = new WebSocket('ws://127.0.0.1:$LOCAL_TUNNEL_PORT');
ws.on('open', () => {
  ws.send(JSON.stringify({type:'ping'}));
});
let pinged = false;
ws.on('message', (d) => {
  const m = JSON.parse(d.toString());
  if (m.type === 'pong' && !pinged) {
    pinged = true;
    ws.send(JSON.stringify({
      type: 'command',
      session_id: '',
      provider: 'claude',
      command: 'What is 2+2? Reply with just the number.',
      options: { cwd: '$REMOTE_CWD', permissionMode: 'dangerouslySkipPermissions' }
    }));
  } else if (m.type === 'provider-message') {
    const data = m.data;
    if (data && data.type === 'result' && data.result) {
      console.log('RESULT:' + (typeof data.result === 'string' ? data.result : JSON.stringify(data.result)));
    } else if (data && data.type === 'assistant' && data.content) {
      for (const b of data.content) {
        if (b.type === 'text') console.log('TEXT:' + b.text.substring(0, 200));
      }
    }
  } else if (m.type === 'complete') {
    console.log('EXIT:' + m.exit_code);
    ws.close();
    process.exit(m.exit_code === 0 ? 0 : 1);
  } else if (m.type === 'error') {
    console.log('ERROR:' + m.error);
    ws.close();
    process.exit(1);
  }
});
ws.on('error', (e) => { console.log('WSERROR:' + e.message); process.exit(1); });
setTimeout(() => { console.log('TIMEOUT'); process.exit(1); }, 120000);
" 2>&1)

EXIT_CODE=$?
if echo "$CLAUDE_RESULT" | grep -q "EXIT:0"; then
  assert "Claude command completed (exit 0)" "true"
else
  assert "Claude command completed (exit 0)" "false"
fi

if echo "$CLAUDE_RESULT" | grep -qE "(RESULT:|TEXT:)"; then
  assert "Got Claude output" "true"
  echo "    Output: $(echo "$CLAUDE_RESULT" | grep -E '(RESULT:|TEXT:)' | head -1)"
else
  assert "Got Claude output" "false"
  echo "    Raw: $(echo "$CLAUDE_RESULT" | head -3)"
fi

# ---- Test 7: Project Directory Access ----
bold "[7] Remote Project Directory"
DIR_EXISTS=$(ssh "$REMOTE_USER@$REMOTE_HOST" "[ -d '$REMOTE_CWD' ] && echo yes || echo no")
assert "Project directory $REMOTE_CWD exists" "$([ "$DIR_EXISTS" = "yes" ] && echo true || echo false)"

FILE_COUNT=$(ssh "$REMOTE_USER@$REMOTE_HOST" "ls '$REMOTE_CWD' 2>/dev/null | wc -l")
assert "Project has files ($FILE_COUNT entries)" "$([ "$FILE_COUNT" -gt 0 ] && echo true || echo false)"

# ---- Summary ----
echo ""
bold "========================================="
bold "  Results: $PASS passed, $FAIL failed"
bold "========================================="

[ "$FAIL" -eq 0 ] && exit 0 || exit 1

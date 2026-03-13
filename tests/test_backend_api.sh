#!/bin/bash
# Backend REST API Integration Tests
# Prerequisites: backend running on localhost:3001
#
# Usage: ./tests/test_backend_api.sh

set -e

BACKEND_URL="${BACKEND_URL:-http://localhost:3001}"
PASS=0
FAIL=0
TOKEN=""

green() { printf "\033[32m%s\033[0m\n" "$1"; }
red()   { printf "\033[31m%s\033[0m\n" "$1"; }
bold()  { printf "\033[1m%s\033[0m\n" "$1"; }

assert_status() {
  local name="$1" expected="$2" actual="$3"
  if [ "$actual" -eq "$expected" ]; then
    green "  PASS: $name (HTTP $actual)"
    PASS=$((PASS + 1))
  else
    red "  FAIL: $name (expected $expected, got $actual)"
    FAIL=$((FAIL + 1))
  fi
}

assert_json_field() {
  local name="$1" body="$2" field="$3"
  local val
  val=$(echo "$body" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('$field','__MISSING__'))" 2>/dev/null || echo "__ERROR__")
  if [ "$val" != "__MISSING__" ] && [ "$val" != "__ERROR__" ]; then
    green "  PASS: $name (field '$field' present)"
    PASS=$((PASS + 1))
  else
    red "  FAIL: $name (field '$field' missing)"
    FAIL=$((FAIL + 1))
  fi
}

bold "========================================="
bold "  Backend REST API Integration Tests"
bold "========================================="
echo ""

# ---- Health Check ----
bold "[1] Health Check"
HTTP_CODE=$(curl -s -o /tmp/test_body.json -w "%{http_code}" "$BACKEND_URL/api/health")
BODY=$(cat /tmp/test_body.json)
assert_status "GET /api/health" 200 "$HTTP_CODE"
assert_json_field "Health response has 'status'" "$BODY" "status"

# ---- Auth: Login or Register ----
bold "[2] Auth - Login or Register"

# Try login with existing admin user first
HTTP_CODE=$(curl -s -o /tmp/test_body.json -w "%{http_code}" \
  -X POST "$BACKEND_URL/api/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin123"}')
BODY=$(cat /tmp/test_body.json)
TOKEN=$(echo "$BODY" | python3 -c "import sys,json; print(json.load(sys.stdin).get('token',''))" 2>/dev/null)

if [ -n "$TOKEN" ] && [ "$TOKEN" != "" ] && [ "$TOKEN" != "None" ]; then
  assert_status "POST /api/auth/login (admin)" 200 "$HTTP_CODE"
  assert_json_field "Login returns token" "$BODY" "token"
else
  # No existing user, try register
  HTTP_CODE=$(curl -s -o /tmp/test_body.json -w "%{http_code}" \
    -X POST "$BACKEND_URL/api/auth/register" \
    -H "Content-Type: application/json" \
    -d '{"username":"admin","password":"admin123"}')
  BODY=$(cat /tmp/test_body.json)
  assert_status "POST /api/auth/register" 200 "$HTTP_CODE"
  assert_json_field "Register returns token" "$BODY" "token"
  TOKEN=$(echo "$BODY" | python3 -c "import sys,json; print(json.load(sys.stdin).get('token',''))" 2>/dev/null)
fi

if [ -z "$TOKEN" ] || [ "$TOKEN" = "" ] || [ "$TOKEN" = "None" ]; then
  red "  FATAL: No token obtained, cannot continue authenticated tests"
  echo ""
  echo "Results: $PASS passed, $FAIL failed"
  exit 1
fi

AUTH="Authorization: Bearer $TOKEN"

# ---- Auth: Wrong Password ----
bold "[3] Auth - Wrong Password"
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
  -X POST "$BACKEND_URL/api/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"wrongpassword"}')
assert_status "POST /api/auth/login (wrong pw)" 401 "$HTTP_CODE"

# ---- Auth: Unauthenticated ----
bold "[5] Auth - Unauthenticated Access"
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "$BACKEND_URL/api/projects")
assert_status "GET /api/projects (no token)" 401 "$HTTP_CODE"

# ---- Projects ----
bold "[6] Projects"
HTTP_CODE=$(curl -s -o /tmp/test_body.json -w "%{http_code}" \
  "$BACKEND_URL/api/projects" -H "$AUTH")
assert_status "GET /api/projects" 200 "$HTTP_CODE"

# ---- API Keys CRUD ----
bold "[7] API Keys"
HTTP_CODE=$(curl -s -o /tmp/test_body.json -w "%{http_code}" \
  -X POST "$BACKEND_URL/api/settings/api-keys" \
  -H "Content-Type: application/json" -H "$AUTH" \
  -d '{"keyName":"test-key-'$$'"}')
BODY=$(cat /tmp/test_body.json)
assert_status "POST /api/settings/api-keys (create)" 200 "$HTTP_CODE"
KEY_ID=$(echo "$BODY" | python3 -c "import sys,json; print(json.load(sys.stdin).get('key',{}).get('id',''))" 2>/dev/null)

HTTP_CODE=$(curl -s -o /tmp/test_body.json -w "%{http_code}" \
  "$BACKEND_URL/api/settings/api-keys" -H "$AUTH")
assert_status "GET /api/settings/api-keys (list)" 200 "$HTTP_CODE"

if [ -n "$KEY_ID" ] && [ "$KEY_ID" != "" ]; then
  HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
    -X DELETE "$BACKEND_URL/api/settings/api-keys/$KEY_ID" -H "$AUTH")
  assert_status "DELETE /api/settings/api-keys/:id" 200 "$HTTP_CODE"
fi

# ---- Credentials CRUD ----
bold "[8] Credentials"
HTTP_CODE=$(curl -s -o /tmp/test_body.json -w "%{http_code}" \
  -X POST "$BACKEND_URL/api/settings/credentials" \
  -H "Content-Type: application/json" -H "$AUTH" \
  -d '{"credentialName":"test-cred","credentialType":"github","credentialValue":"ghp_test","description":"test"}')
BODY=$(cat /tmp/test_body.json)
assert_status "POST /api/settings/credentials (create)" 200 "$HTTP_CODE"
CRED_ID=$(echo "$BODY" | python3 -c "import sys,json; print(json.load(sys.stdin).get('credential',{}).get('id',''))" 2>/dev/null)

HTTP_CODE=$(curl -s -o /tmp/test_body.json -w "%{http_code}" \
  "$BACKEND_URL/api/settings/credentials" -H "$AUTH")
assert_status "GET /api/settings/credentials (list)" 200 "$HTTP_CODE"

if [ -n "$CRED_ID" ] && [ "$CRED_ID" != "" ]; then
  HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
    -X DELETE "$BACKEND_URL/api/settings/credentials/$CRED_ID" -H "$AUTH")
  assert_status "DELETE /api/settings/credentials/:id" 200 "$HTTP_CODE"
fi

# ---- User Git Config ----
bold "[9] User Git Config"
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
  -X POST "$BACKEND_URL/api/user/git-config" \
  -H "Content-Type: application/json" -H "$AUTH" \
  -d '{"gitName":"Test User","gitEmail":"test@example.com"}')
assert_status "POST /api/user/git-config" 200 "$HTTP_CODE"

HTTP_CODE=$(curl -s -o /tmp/test_body.json -w "%{http_code}" \
  "$BACKEND_URL/api/user/git-config" -H "$AUTH")
assert_status "GET /api/user/git-config" 200 "$HTTP_CODE"

# ---- Remote Servers CRUD ----
bold "[10] Remote Servers"
HTTP_CODE=$(curl -s -o /tmp/test_body.json -w "%{http_code}" \
  -X POST "$BACKEND_URL/api/remote-servers" \
  -H "Content-Type: application/json" -H "$AUTH" \
  -d '{"name":"test-server-'$$'","hostname":"192.168.1.1","sshPort":22,"sshUser":"user","brokerPort":9999}')
BODY=$(cat /tmp/test_body.json)
assert_status "POST /api/remote-servers (create)" 200 "$HTTP_CODE"
SERVER_ID=$(echo "$BODY" | python3 -c "import sys,json; print(json.load(sys.stdin).get('server',{}).get('id',''))" 2>/dev/null)

HTTP_CODE=$(curl -s -o /tmp/test_body.json -w "%{http_code}" \
  "$BACKEND_URL/api/remote-servers" -H "$AUTH")
assert_status "GET /api/remote-servers (list)" 200 "$HTTP_CODE"

if [ -n "$SERVER_ID" ] && [ "$SERVER_ID" != "" ]; then
  HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
    -X PUT "$BACKEND_URL/api/remote-servers/$SERVER_ID" \
    -H "Content-Type: application/json" -H "$AUTH" \
    -d '{"name":"updated-server-'$$'"}')
  assert_status "PUT /api/remote-servers/:id (update)" 200 "$HTTP_CODE"

  HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
    -X DELETE "$BACKEND_URL/api/remote-servers/$SERVER_ID" -H "$AUTH")
  assert_status "DELETE /api/remote-servers/:id" 200 "$HTTP_CODE"
fi

# ---- Onboarding ----
bold "[11] Onboarding"
HTTP_CODE=$(curl -s -o /tmp/test_body.json -w "%{http_code}" \
  "$BACKEND_URL/api/user/onboarding" -H "$AUTH")
assert_status "GET /api/user/onboarding" 200 "$HTTP_CODE"

HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
  -X POST "$BACKEND_URL/api/user/onboarding/complete" -H "$AUTH")
assert_status "POST /api/user/onboarding/complete" 200 "$HTTP_CODE"

# ---- Summary ----
echo ""
bold "========================================="
bold "  Results: $PASS passed, $FAIL failed"
bold "========================================="

[ "$FAIL" -eq 0 ] && exit 0 || exit 1

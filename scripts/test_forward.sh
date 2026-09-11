#!/usr/bin/env bash
set -euo pipefail

SERVER_URL="${1:-http://localhost:8080}"
DEVICE_TOKEN="${2:-}"
RECIPIENT="${3:-+15551234567}"
SENDER="${4:-BankSecure}"
BODY="${5:-Your verification code is 739201. Valid for 10 minutes.}"

echo "📡 Forwarding SMS to $SERVER_URL/api/v1/sms/forward..."

curl -s -X POST "$SERVER_URL/api/v1/sms/forward" \
  -H "Content-Type: application/json" \
  ${DEVICE_TOKEN:+-H "X-Device-Token: $DEVICE_TOKEN"} \
  -d "{
    \"recipient_number\": \"$RECIPIENT\",
    \"sender\": \"$SENDER\",
    \"body\": \"$BODY\",
    \"device_id\": \"cli_simulator\",
    \"sim_slot\": 0
  }" | jq . || cat
echo ""

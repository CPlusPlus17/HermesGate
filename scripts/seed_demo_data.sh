#!/usr/bin/env bash
set -euo pipefail

SERVER_URL="${1:-http://localhost:8080}"
ADMIN_TOKEN="${2:-}"

if [ -z "$ADMIN_TOKEN" ]; then
  echo "Usage: $0 [SERVER_URL] <ADMIN_TOKEN>"
  echo "Example: $0 http://localhost:8080 sms_adm_..."
  exit 1
fi

echo "🌱 Seeding demo phone numbers, devices, and messages..."

# Register phone numbers
echo "1. Registering phone numbers..."
curl -s -X POST "$SERVER_URL/api/v1/admin/numbers" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{"phone_number": "+15551234567", "label": "US Work Line (SIM 1)"}' > /dev/null

curl -s -X POST "$SERVER_URL/api/v1/admin/numbers" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{"phone_number": "+41791234567", "label": "Swiss Personal eSIM"}' > /dev/null

curl -s -X POST "$SERVER_URL/api/v1/admin/numbers" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{"phone_number": "+447700900123", "label": "UK Banking Line"}' > /dev/null

# Register devices
echo "2. Registering demo devices..."
DEV_RESP=$(curl -s -X POST "$SERVER_URL/api/v1/admin/devices" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{"name": "Pixel 8 Pro (Primary)", "platform": "android"}')
DEV_TOKEN=$(echo "$DEV_RESP" | jq -r '.token // ""')

# Ingest sample SMS messages with various OTP formats
echo "3. Ingesting sample SMS messages..."
curl -s -X POST "$SERVER_URL/api/v1/sms/forward" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{
    "recipient_number": "+15551234567",
    "sender": "Google",
    "body": "G-491823 is your Google verification code.",
    "sim_slot": 0
  }' > /dev/null

curl -s -X POST "$SERVER_URL/api/v1/sms/forward" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{
    "recipient_number": "+15551234567",
    "sender": "WhatsApp",
    "body": "Your WhatsApp code: 839-204. Do not share this code.",
    "sim_slot": 0
  }' > /dev/null

curl -s -X POST "$SERVER_URL/api/v1/sms/forward" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{
    "recipient_number": "+41791234567",
    "sender": "PostFinance",
    "body": "Security code: 719283 for e-finance payment authorization.",
    "sim_slot": 1
  }' > /dev/null

curl -s -X POST "$SERVER_URL/api/v1/sms/forward" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{
    "recipient_number": "+447700900123",
    "sender": "Barclays",
    "body": "Your passcode is 582910 to authorize your recent transaction.",
    "sim_slot": 0
  }' > /dev/null

# Create AI MCP tokens with isolated permissions
echo "4. Creating granular AI tokens..."
# Token for US Work Line ONLY
US_AI=$(curl -s -X POST "$SERVER_URL/api/v1/admin/tokens" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{"name": "US Line AI Agent", "allowed_numbers": ["+15551234567"], "can_read": true, "can_delete": false}')
US_SECRET=$(echo "$US_AI" | jq -r '.secret // ""')

# Token for Swiss Line ONLY
CH_AI=$(curl -s -X POST "$SERVER_URL/api/v1/admin/tokens" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{"name": "Swiss Line AI Agent", "allowed_numbers": ["+41791234567"], "can_read": true, "can_delete": false}')
CH_SECRET=$(echo "$CH_AI" | jq -r '.secret // ""')

echo ""
echo "✅ Demo data seeded successfully!"
echo "---------------------------------------------------------"
echo "📱 US Line AI Secret (access only to +15551234567):"
echo "   $US_SECRET"
echo "📱 Swiss Line AI Secret (access only to +41791234567):"
echo "   $CH_SECRET"
echo "📲 Device Pairing Token:"
echo "   $DEV_TOKEN"
echo "---------------------------------------------------------"

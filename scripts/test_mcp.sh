#!/usr/bin/env bash
set -euo pipefail

SERVER_URL="${1:-http://localhost:8080}"
TOKEN="${2:-}"

if [ -z "$TOKEN" ]; then
  echo "Usage: $0 [SERVER_URL] <TOKEN>"
  exit 1
fi

echo "=== 1. Testing MCP initialize ==="
curl -s -X POST "$SERVER_URL/mcp/messages?token=$TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | jq .

echo ""
echo "=== 2. Testing MCP tools/list ==="
curl -s -X POST "$SERVER_URL/mcp/messages?token=$TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' | jq .

echo ""
echo "=== 3. Testing MCP tools/call (list_allowed_numbers) ==="
curl -s -X POST "$SERVER_URL/mcp/messages?token=$TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"list_allowed_numbers","arguments":{}}}' | jq .

echo ""
echo "=== 4. Testing MCP tools/call (get_latest_otp) ==="
curl -s -X POST "$SERVER_URL/mcp/messages?token=$TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"get_latest_otp","arguments":{"max_age_minutes":60}}}' | jq .

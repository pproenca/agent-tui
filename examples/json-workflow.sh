#!/bin/bash
# JSON output workflow example
# Demonstrates using -f json for programmatic interaction

set -e

echo "=== JSON Output Workflow ==="
echo

# Start a simple session
echo "1. Starting session..."
SPAWN_RESULT=$(agent-tui spawn bash -f json)
SESSION_ID=$(echo "$SPAWN_RESULT" | jq -r '.session_id')
PID=$(echo "$SPAWN_RESULT" | jq -r '.pid')
echo "   Session: $SESSION_ID"
echo "   PID: $PID"

# Type something to create content
echo "2. Creating some content..."
agent-tui type "ls -la"
agent-tui press Enter
sleep 1

# Get snapshot as JSON
echo "3. Getting JSON snapshot with elements..."
SNAPSHOT=$(agent-tui snapshot -i -f json)

# Parse snapshot data
SCREEN_SIZE=$(echo "$SNAPSHOT" | jq -r '.size | "\(.cols)x\(.rows)"')
ELEMENT_COUNT=$(echo "$SNAPSHOT" | jq '.elements | length // 0')
echo "   Screen size: $SCREEN_SIZE"
echo "   Elements detected: $ELEMENT_COUNT"

# Get sessions as JSON
echo "4. Getting session list..."
SESSIONS=$(agent-tui sessions -f json)
ACTIVE=$(echo "$SESSIONS" | jq -r '.active_session // "none"')
TOTAL=$(echo "$SESSIONS" | jq '.sessions | length')
echo "   Active session: $ACTIVE"
echo "   Total sessions: $TOTAL"

# Get health as JSON
echo "5. Getting health status..."
HEALTH=$(agent-tui health -f json)
STATUS=$(echo "$HEALTH" | jq -r '.status')
UPTIME=$(echo "$HEALTH" | jq -r '.uptime_ms')
echo "   Status: $STATUS"
echo "   Uptime: ${UPTIME}ms"

# Kill session using JSON output to verify
echo "6. Killing session..."
KILL_RESULT=$(agent-tui kill -f json)
SUCCESS=$(echo "$KILL_RESULT" | jq -r '.success')
echo "   Success: $SUCCESS"

echo
echo "=== Workflow Complete ==="

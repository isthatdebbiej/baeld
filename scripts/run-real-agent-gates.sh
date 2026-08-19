#!/usr/bin/env bash
set -euo pipefail

: "${OPENAI_BASE_URL:?Set an OpenAI-compatible endpoint}"
: "${OPENAI_API_KEY:?Set the endpoint credential}"
: "${BAELD_MODEL:?Set the pinned OpenAI-compatible model id without provider prefix}"

export BAELD_BASE_URL="${BAELD_BASE_URL:-http://127.0.0.1:4173}"
export PORT="${BAELD_BASE_URL##*:}"
bun workloads/server.mjs > /tmp/baeld-real-agent-server.log 2>&1 &
server_pid=$!
trap 'kill "$server_pid" 2>/dev/null || true; wait "$server_pid" 2>/dev/null || true' EXIT

for _ in {1..100}; do
  curl -fsS "$BAELD_BASE_URL/api/state?session=health" >/dev/null && break
  sleep 0.1
done
curl -fsS "$BAELD_BASE_URL/api/state?session=health" >/dev/null

for block in {1..5}; do
  echo "real-agent block $block/5: Playwright"
  scripts/run-scoped.sh run --config experiments/real-agent.toml -- node workloads/driver/real_playwright.mjs
  echo "real-agent block $block/5: Stagehand"
  scripts/run-scoped.sh run --config experiments/real-agent.toml -- bun workloads/driver/real_stagehand.mjs
  echo "real-agent block $block/5: Browser Use"
  PYTHONPATH=packages/python/src scripts/run-scoped.sh run --config experiments/real-agent.toml -- python workloads/driver/real_browser_use.py
done

target/release/baeld cleanup

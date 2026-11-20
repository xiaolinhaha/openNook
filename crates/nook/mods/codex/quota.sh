#!/bin/sh
# Ask Codex's local app-server for the authenticated account's read-only usage snapshot.
{
  printf '%s\n' '{"method":"initialize","id":1,"params":{"clientInfo":{"name":"opennook-codex-monitor","title":"openNook Codex Monitor","version":"1.0.0"},"capabilities":{"experimentalApi":true}}}'
  printf '%s\n' '{"method":"initialized","params":{}}'
  printf '%s\n' '{"method":"account/rateLimits/read","id":2,"params":{}}'
  sleep 3
} | codex app-server --listen stdio:// 2>/dev/null

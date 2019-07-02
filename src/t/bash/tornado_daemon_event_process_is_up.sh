#!/usr/bin/env bash

TEXT="NotMatched"

if curl -H "content-type: application/json" -X POST -d '{"event":{"type":"something", "created_ms":111, "payload": {}}, "process_type":"SkipActions"}' http://localhost:4748/api/send_event | grep -q "$TEXT"; then
    echo "Tornado daemon event sending check succeeded"
else
    echo "[-] FAILED: Tornado daemon event sending check failed"
    exit 1
fi
#!/usr/bin/env bash

HTTP_RESPONSE_CODE=$(curl -sS -k -o /dev/null -w "%{http_code}" -H "content-type: application/json" -X POST -d '{"event":{"type":"something", "created_ms":111, "payload": {}}, "process_type":"SkipActions"}' http://localhost:4748/api/send_event)

if [ $HTTP_RESPONSE_CODE -ne 200 ]; then
    echo "[-] FAILED: Tornado daemon event sending check failed, response code was: $HTTP_RESPONSE_CODE"
    exit 1
else
    echo "Tornado daemon event sending check succeeded"
fi 

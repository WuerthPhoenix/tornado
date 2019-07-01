#!/usr/bin/env bash

HTTP_RESPONSE_CODE=$(curl -sS -k -o /dev/null -w "%{http_code}" http://localhost:4748/monitoring/ping)

if [ $HTTP_RESPONSE_CODE -ne 200 ]; then
    echo "[-] FAILED: Tornado daemon healthcheck failed, response code was: $HTTP_RESPONSE_CODE"
    exit 1
else
    echo "Tornado daemon healthcheck succeeded"
fi 

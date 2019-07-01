#!/usr/bin/env bash

netstat -ln | grep ":8080 " 2>&1 > /dev/null

if [ $? -eq 1 ]; then
    echo "[-] FAILED: Tornado Webhook Collector healthcheck failed"
    exit 1
else
    echo "Tornado Webhook Collector healthcheck succeeded"
fi
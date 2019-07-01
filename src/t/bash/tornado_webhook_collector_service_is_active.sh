#!/usr/bin/env bash

SERVICE=tornado_webhook_collector.service

STATUS_CHECK=`systemctl is-active $SERVICE`
STATUS=${STATUS_CHECK}

if [[ $STATUS == 'active' ]]; then
    echo "Service '$SERVICE' is active"
else
    echo "[-] Service '$SERVICE' healthcheck failed, status was: $STATUS"
    exit 1
fi
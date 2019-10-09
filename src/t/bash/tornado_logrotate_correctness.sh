#!/bin/sh

LOGROTATE_FILE=/etc/logrotate.d/tornado

/usr/sbin/logrotate -s /tmp/logrotate.status $LOGROTATE_FILE

#!/bin/bash

function usage() {
    echo "Usage: $(basename "$0") FILE [-f]"
    echo "    FILE - must contain a tornado event in valid JSON format"
    echo "    -f [optional] Forces also the execution of the triggered actions"
}

if grep "tornado" /etc/hosts >> /dev/null ; then
   TORNADO_HOST="tornado.neteyelocal"
else
   TORNADO_HOST="localhost"
fi
TORNADO_API_PORT=4748

if test -z "$1" ; then
    echo "Path to JSON File missing"
    exit 1
fi

if test "$1" == "-h" || test "$1" == "--help" ; then 
    usage
    exit 0
fi

CONTENT="$(jq . < "$1")"
RET="$?"
if test "$RET" -ne 0 ; then
    echo -e "'$1' does not exist, or does not contain valid JSON $CONTENT"
    exit 2
fi

if test "$2" == "-f" ; then
    PROCESS_TYPE="Full"
else
    PROCESS_TYPE="SkipActions"
fi

RESPONSE="$(curl -sS -H "content-type: application/json" -X POST -d "{ \"process_type\":\"$PROCESS_TYPE\", \"event\":$CONTENT}" "http://$TORNADO_HOST:$TORNADO_API_PORT/api/send_event" )"
RET=$?
if test "$RET" -ne 0 ; then
    echo "$RESPONSE"
    exit 3
fi

echo "$RESPONSE" | jq .


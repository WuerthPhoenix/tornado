#!/bin/bash

INSIDE_GIT="$(git rev-parse --is-inside-work-tree)"


if [[ $INSIDE_GIT == 'true' ]] ; then
    echo "[i] Running locally"
    SCRIPTPATH="$( cd "$(dirname "$0")" ; pwd -P )"
    TEST_ROOT="$(dirname "$(dirname "$(dirname "$SCRIPTPATH")")")"
else
    echo "[!] We currently do not deploy the documentation via RPM"
    exit 0
fi

if grep -RI -E '\.\ \ ' $TEST_ROOT/*.md $TEST_ROOT/**/*.md ; then
    echo "[-] Found Documentation with double space after punctuation"
    exit 1
fi

exit 0

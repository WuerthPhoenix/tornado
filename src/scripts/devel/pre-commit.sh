#!/usr/bin/sh
CARGO_WORKSPACE="$PWD/src"
echo $CARGO_WORKSPACE
cd $CARGO_WORKSPACE
cargo fmt --all -- --check
RETVAL=$?
cd ..
exit $RETVAL

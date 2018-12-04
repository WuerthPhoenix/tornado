#!/usr/bin/sh
CARGO_WORKSPACE="$PWD/src"

cd $CARGO_WORKSPACE
cargo fmt --all -- --check
RETVAL=$?
cd ..

exit $RETVAL

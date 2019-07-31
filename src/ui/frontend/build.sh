#!/usr/bin/env bash


SRC="../dto/pkg/dto.d.ts"
DEST_DIR="./src/generated"
DEST="/dto.ts"

rm -rf $DEST_DIR
mkdir $DEST_DIR
cp -r $SRC $DEST_DIR$DEST

./build-ui.sh
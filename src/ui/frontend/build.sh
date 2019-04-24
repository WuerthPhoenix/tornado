#!/usr/bin/env bash


SRC="../dto/pkg/dto.d.ts"
DEST_DIR="./src/generated"

rm -rf $DEST_DIR
mkdir $DEST_DIR
cp -r $SRC $DEST_DIR

npm run lint && \
npm run test:unit && \
npm run build

#!/usr/bin/env bash


SRC="../dto/pkg/dto.d.ts"
DEST_DIR="./src/generated"

rm -rf $DEST_DIR
mkdir $DEST_DIR
cp -r $SRC $DEST_DIR

rm -rf dist/ && \
rm -rf node_modules/ && \
npm install && \
npm run lint && \
npm run test:unit && \
npm run test:e2e-headless && \
npm run build

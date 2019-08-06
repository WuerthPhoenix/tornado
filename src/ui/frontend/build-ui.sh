#!/usr/bin/env bash

rm -rf dist/ && \
rm -rf node_modules/ && \
npm install && \
npm run lint && \
npm run test:unit && \
npm run test:e2e-headless && \
npm run build

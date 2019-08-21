#!/usr/bin/env bash

echo "---------------------------"
echo "- BUILD DTO"
echo "---------------------------"

cd dto 
./build.sh
cd ..

echo "---------------------------"
echo "- BUILD BACKEND"
echo "---------------------------"

cd backend
cargo build 
cd ..

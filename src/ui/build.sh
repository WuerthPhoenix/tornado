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

echo "---------------------------"
echo "- BUILD FRONTEND"
echo "---------------------------"

cd frontend
./build.sh
cd ..

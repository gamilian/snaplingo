#!/usr/bin/env bash

set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}🚀 启动 SnapLingo 开发模式...${NC}"
echo ""

# Check if node_modules exists
if [ ! -d "node_modules" ]; then
    echo -e "${RED}❌ node_modules not found. Please run: npm install${NC}"
    exit 1
fi

# Native hosts own their windows, input, and platform commands.
export RUST_BACKTRACE=1
exec npm run native:dev

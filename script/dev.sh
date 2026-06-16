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

# Set Rust backtrace for better debugging
export RUST_BACKTRACE=1

# Show developer tools hint based on platform
OS=$(uname -s)
case "$OS" in
    Darwin)
        echo -e "${YELLOW}💡 提示: 按 Cmd+Option+I 打开开发者工具${NC}"
        ;;
    Linux)
        echo -e "${YELLOW}💡 提示: 按 Ctrl+Shift+I 打开开发者工具${NC}"
        ;;
esac

echo ""
echo -e "${GREEN}启动应用中...${NC}"
echo ""

# Start Tauri dev mode
npm run tauri:dev

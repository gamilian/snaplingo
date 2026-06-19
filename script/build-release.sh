#!/usr/bin/env bash

set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Start time
START_TIME=$(date +%s)

echo -e "${GREEN}🏗️  SnapLingo Release Build${NC}"
echo ""

# Check if node_modules exists
if [ ! -d "node_modules" ]; then
    echo -e "${RED}❌ node_modules not found. Please run: npm install${NC}"
    exit 1
fi

# Step 1: Clean old build artifacts
echo -e "${GREEN}🧹 清理旧构建产物...${NC}"
rm -rf dist/
rm -rf target/release
# Clean up any temporary DMG files from previous builds
rm -f target/release/bundle/macos/rw.*.dmg 2>/dev/null || true
rm -f target/release/bundle/dmg/rw.*.dmg 2>/dev/null || true
echo -e "   清理完成"
echo ""

# Step 2: Version check
echo -e "${GREEN}📦 版本检查...${NC}"
PACKAGE_VERSION=$(grep '"version"' package.json | head -1 | sed 's/.*: "\(.*\)".*/\1/')
CARGO_VERSION=$(grep '^version = ' src-tauri/Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')

echo -e "   package.json: ${YELLOW}${PACKAGE_VERSION}${NC}"
echo -e "   Cargo.toml:   ${YELLOW}${CARGO_VERSION}${NC}"

if [ "$PACKAGE_VERSION" != "$CARGO_VERSION" ]; then
    echo -e "${YELLOW}⚠️  警告: 版本号不一致！${NC}"
else
    echo -e "   ✅ 版本号一致"
fi
echo ""

# Step 3: Build frontend
echo -e "${GREEN}🔨 构建前端...${NC}"
npm run build
echo -e "   前端构建完成"
echo ""

# Step 4: Build Tauri release
echo -e "${GREEN}🦀 构建 Tauri Release...${NC}"
npm run tauri:build
echo ""

# Step 5: Verify build artifacts
echo -e "${GREEN}✅ 验证构建产物...${NC}"

BUNDLE_DIR="target/release/bundle"
if [ ! -d "$BUNDLE_DIR" ]; then
    echo -e "${RED}❌ 构建失败: 未找到 bundle 目录${NC}"
    exit 1
fi

# Detect platform
OS=$(uname -s)
case "$OS" in
    Darwin)
        echo -e "   平台: ${YELLOW}macOS${NC}"
        APP_PATH="$BUNDLE_DIR/macos/SnapLingo.app"
        DMG_PATH=$(find "$BUNDLE_DIR/dmg" -name "SnapLingo_*.dmg" -not -name "rw.*" 2>/dev/null | head -1)

        if [ -d "$APP_PATH" ]; then
            APP_SIZE=$(du -sh "$APP_PATH" | cut -f1)
            echo -e "   📦 App: $APP_PATH (${APP_SIZE})"
        fi

        if [ -n "$DMG_PATH" ]; then
            DMG_SIZE=$(du -sh "$DMG_PATH" | cut -f1)
            DMG_FILENAME=$(basename "$DMG_PATH")
            echo -e "   💿 DMG: $DMG_FILENAME (${DMG_SIZE})"
        fi
        ;;
    Linux)
        echo -e "   平台: ${YELLOW}Linux${NC}"
        APPIMAGE_PATH=$(find "$BUNDLE_DIR/appimage" -name "*.AppImage" 2>/dev/null | head -1)
        DEB_PATH=$(find "$BUNDLE_DIR/deb" -name "*.deb" 2>/dev/null | head -1)

        if [ -n "$APPIMAGE_PATH" ]; then
            APPIMAGE_SIZE=$(du -sh "$APPIMAGE_PATH" | cut -f1)
            echo -e "   📦 AppImage: $APPIMAGE_PATH (${APPIMAGE_SIZE})"
        fi

        if [ -n "$DEB_PATH" ]; then
            DEB_SIZE=$(du -sh "$DEB_PATH" | cut -f1)
            echo -e "   📦 DEB: $DEB_PATH (${DEB_SIZE})"
        fi
        ;;
esac

echo ""
echo -e "   所有构建产物位于: ${YELLOW}$BUNDLE_DIR${NC}"
echo ""

# Calculate build time
END_TIME=$(date +%s)
BUILD_TIME=$((END_TIME - START_TIME))
MINUTES=$((BUILD_TIME / 60))
SECONDS=$((BUILD_TIME % 60))

echo -e "${GREEN}🎉 构建完成！${NC}"
echo -e "   总耗时: ${YELLOW}${MINUTES}m ${SECONDS}s${NC}"
echo ""

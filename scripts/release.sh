#!/bin/bash
# Semantic Release Script for Agenterra
# Usage: ./scripts/release.sh [patch|minor|major|alpha|beta|rc]

set -e

RELEASE_TYPE=${1:-patch}

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}🚀 Agenterra Semantic Release${NC}"
echo -e "Release type: ${YELLOW}${RELEASE_TYPE}${NC}"

# Check if cargo-release is installed
if ! command -v cargo-release &> /dev/null; then
    echo -e "${YELLOW}📦 Installing cargo-release...${NC}"
    cargo install cargo-release
fi

Validate current state
echo -e "${YELLOW}🔍 Validating repository state...${NC}"
if [[ -n $(git status --porcelain) ]]; then
    echo -e "${RED}❌ Working directory is not clean. Please commit or stash changes.${NC}"
    exit 1
fi

# Check if we're on main branch
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [[ "$CURRENT_BRANCH" != "main" ]]; then
    echo -e "${RED}❌ Not on main branch. Please switch to main before releasing.${NC}"
    exit 1
fi

# Pull latest changes
echo -e "${YELLOW}📥 Pulling latest changes...${NC}"
git pull origin main

# Confirm release
echo -e "${YELLOW}❓ Ready to release. Continue? (y/N)${NC}"
read -r CONFIRM
if [[ ! "$CONFIRM" =~ ^[Yy]$ ]]; then
    echo -e "${YELLOW}📦 Release cancelled.${NC}"
    exit 0
fi

# Execute release
echo -e "${GREEN}🚀 Executing semantic release...${NC}"
cargo release --execute "$RELEASE_TYPE"

# Get the new version tag
NEW_TAG=$(git describe --tags --exact-match HEAD)
echo -e "${GREEN}✅ Released ${NEW_TAG}${NC}"
echo -e "${YELLOW}🏗️  GitHub Actions will now build and publish release binaries.${NC}"
echo -e "${YELLOW}📦 Check: https://github.com/clafollett/agenterra/releases${NC}"
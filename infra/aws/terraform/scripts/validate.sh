#!/bin/bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TERRAFORM_DIR="$(dirname "$SCRIPT_DIR")"

cd "$TERRAFORM_DIR"

print_status "Validating Terraform configuration..."

# Check Terraform version
print_status "Checking Terraform version..."
TERRAFORM_VERSION=$(terraform version -json | jq -r '.terraform_version')
print_status "Terraform version: $TERRAFORM_VERSION"

# Format check
print_status "Checking Terraform formatting..."
if terraform fmt -check -recursive; then
    print_status "✓ Terraform files are properly formatted"
else
    print_warning "Some files need formatting. Run: terraform fmt -recursive"
fi

# Validate configuration
print_status "Validating Terraform configuration..."
terraform validate

print_status ""
print_status "=============================================="
print_status "  VALIDATION COMPLETE"
print_status "=============================================="
print_status ""
print_status "All checks passed! Configuration is valid."
print_status ""

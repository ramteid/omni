#!/bin/bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
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

print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

usage() {
    echo "Usage: $0 [options]"
    echo ""
    echo "Options:"
    echo "  -p, --plan      Run terraform plan only (no apply)"
    echo "  -y, --yes       Auto-approve terraform apply"
    echo "  -d, --destroy   Destroy infrastructure"
    echo "  -h, --help      Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0              # Interactive deployment"
    echo "  $0 --plan       # Show what would be deployed"
    echo "  $0 --yes        # Deploy without confirmation"
    echo "  $0 --destroy    # Destroy infrastructure"
    exit 1
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TERRAFORM_DIR="$(dirname "$SCRIPT_DIR")"

cd "$TERRAFORM_DIR"

# Parse arguments
PLAN_ONLY=false
AUTO_APPROVE=false
DESTROY=false

while [[ $# -gt 0 ]]; do
    case $1 in
        -p|--plan)
            PLAN_ONLY=true
            shift
            ;;
        -y|--yes)
            AUTO_APPROVE=true
            shift
            ;;
        -d|--destroy)
            DESTROY=true
            shift
            ;;
        -h|--help)
            usage
            ;;
        *)
            print_error "Unknown option: $1"
            usage
            ;;
    esac
done

print_status "Starting Omni deployment..."

# Check if terraform.tfvars exists
if [ ! -f "terraform.tfvars" ]; then
    print_error "terraform.tfvars not found!"
    print_info "Copy the example file and configure it:"
    print_info "  cp terraform.tfvars.example terraform.tfvars"
    print_info "  # Edit terraform.tfvars with your values"
    exit 1
fi

# Check if backend is configured
if [ ! -f "backend.tf" ]; then
    print_warning "backend.tf not found - using local state"
    print_info "For production use, configure remote state:"
    print_info "  ./scripts/init-backend.sh"
    print_info "  cp backend.tf.example backend.tf"
    print_info "  # Edit backend.tf with your account ID"
    print_info "  terraform init"
    echo ""
    read -p "Continue with local state? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Initialize Terraform
print_status "Initializing Terraform..."
terraform init -upgrade

# Format check
print_status "Formatting Terraform files..."
terraform fmt -recursive

# Validate configuration
print_status "Validating configuration..."
terraform validate

if [ "$DESTROY" = true ]; then
    print_warning "⚠️  DESTROY MODE - This will delete all infrastructure!"
    print_warning ""

    if [ "$AUTO_APPROVE" = false ]; then
        read -p "Are you sure you want to destroy all resources? (yes/NO) " -r
        echo
        if [[ ! $REPLY = "yes" ]]; then
            print_status "Destroy cancelled."
            exit 0
        fi
    fi

    print_status "Running terraform destroy..."
    if [ "$AUTO_APPROVE" = true ]; then
        terraform destroy -auto-approve
    else
        terraform destroy
    fi

    print_status "Infrastructure destroyed."
    exit 0
fi

# Run terraform plan
print_status "Running terraform plan..."
terraform plan -out=tfplan

if [ "$PLAN_ONLY" = true ]; then
    print_status "Plan complete. Review the output above."
    print_info "To apply these changes, run: $0 --yes"
    rm -f tfplan
    exit 0
fi

# Apply changes
print_status "Applying Terraform changes..."

if [ "$AUTO_APPROVE" = true ]; then
    terraform apply tfplan
else
    terraform apply tfplan
fi

rm -f tfplan

# Get outputs
print_status ""
print_status "=============================================="
print_status "  DEPLOYMENT COMPLETE"
print_status "=============================================="
print_status ""

terraform output -raw next_steps

print_status ""
print_status "Deployment completed successfully!"

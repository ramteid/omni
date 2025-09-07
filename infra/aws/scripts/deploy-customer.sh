#!/bin/bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Script configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INFRA_DIR="$(dirname "$SCRIPT_DIR")"
CF_TEMPLATE="$INFRA_DIR/cloudformation/omni-infrastructure.yaml"
PARAMETERS_FILE="$INFRA_DIR/parameters/default.json"

# Function to print colored output
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to show usage
usage() {
    echo "Usage: $0 <customer-account-id> <region> <external-id> [customer-name] [github-org]"
    echo ""
    echo "Arguments:"
    echo "  customer-account-id    AWS Account ID of the customer"
    echo "  region                 AWS region (e.g., us-east-1)"
    echo "  external-id           External ID provided to customer"
    echo "  customer-name         Customer name for resource naming (optional, default: customer)"
    echo "  github-org            GitHub organization for container images (optional, default: your-github-org)"
    echo ""
    echo "Example:"
    echo "  $0 123456789012 us-east-1 abc123xyz789 acme-corp mycompany"
    exit 1
}

# Validate arguments
if [ $# -lt 3 ]; then
    print_error "Missing required arguments"
    usage
fi

CUSTOMER_ACCOUNT_ID="$1"
REGION="$2"
EXTERNAL_ID="$3"
CUSTOMER_NAME="${4:-customer}"
GITHUB_ORG="${5:-your-github-org}"

# Validate account ID format
if ! [[ "$CUSTOMER_ACCOUNT_ID" =~ ^[0-9]{12}$ ]]; then
    print_error "Invalid AWS Account ID format: $CUSTOMER_ACCOUNT_ID"
    exit 1
fi

# Validate region format
if ! [[ "$REGION" =~ ^[a-z0-9-]+$ ]]; then
    print_error "Invalid AWS region format: $REGION"
    exit 1
fi

# Check if required files exist
if [ ! -f "$CF_TEMPLATE" ]; then
    print_error "CloudFormation template not found: $CF_TEMPLATE"
    exit 1
fi

if [ ! -f "$PARAMETERS_FILE" ]; then
    print_error "Parameters file not found: $PARAMETERS_FILE"
    exit 1
fi

# Set stack name
STACK_NAME="omni-${CUSTOMER_NAME}-infrastructure"

print_status "Starting Omni deployment for customer: $CUSTOMER_NAME"
print_status "Account ID: $CUSTOMER_ACCOUNT_ID"
print_status "Region: $REGION"
print_status "Stack Name: $STACK_NAME"

# Assume the customer's deployment role
ROLE_ARN="arn:aws:iam::${CUSTOMER_ACCOUNT_ID}:role/OmniDeploymentRole"

print_status "Assuming customer deployment role: $ROLE_ARN"

# Get temporary credentials
CREDENTIALS=$(aws sts assume-role \
    --role-arn "$ROLE_ARN" \
    --role-session-name "omni-deployment-$(date +%s)" \
    --external-id "$EXTERNAL_ID" \
    --query 'Credentials.[AccessKeyId,SecretAccessKey,SessionToken]' \
    --output text)

if [ $? -ne 0 ]; then
    print_error "Failed to assume role. Check your AWS credentials and external ID."
    exit 1
fi

# Parse credentials
AWS_ACCESS_KEY_ID=$(echo $CREDENTIALS | cut -d' ' -f1)
AWS_SECRET_ACCESS_KEY=$(echo $CREDENTIALS | cut -d' ' -f2)
AWS_SESSION_TOKEN=$(echo $CREDENTIALS | cut -d' ' -f3)

# Export credentials
export AWS_ACCESS_KEY_ID
export AWS_SECRET_ACCESS_KEY
export AWS_SESSION_TOKEN
export AWS_DEFAULT_REGION="$REGION"

print_status "Successfully assumed customer role"

# Prepare CloudFormation parameters
PARAMETERS="ParameterKey=CustomerName,ParameterValue=$CUSTOMER_NAME"
PARAMETERS="$PARAMETERS ParameterKey=GitHubOrg,ParameterValue=$GITHUB_ORG"

# Add parameters from file if it exists and has content
if [ -s "$PARAMETERS_FILE" ]; then
    print_status "Using parameters from: $PARAMETERS_FILE"
    PARAMETERS="$PARAMETERS --parameters file://$PARAMETERS_FILE"
else
    print_warning "Parameters file is empty, using defaults"
    PARAMETERS="--parameters $PARAMETERS"
fi

# Check if stack already exists
print_status "Checking if stack already exists..."
if aws cloudformation describe-stacks --stack-name "$STACK_NAME" --region "$REGION" >/dev/null 2>&1; then
    print_status "Stack exists, updating..."
    OPERATION="update-stack"
    WAIT_CONDITION="stack-update-complete"
else
    print_status "Stack does not exist, creating..."
    OPERATION="create-stack"
    WAIT_CONDITION="stack-create-complete"
fi

# Deploy the stack
print_status "Deploying CloudFormation stack..."
aws cloudformation $OPERATION \
    --stack-name "$STACK_NAME" \
    --template-body "file://$CF_TEMPLATE" \
    $PARAMETERS \
    --capabilities CAPABILITY_NAMED_IAM \
    --region "$REGION"

if [ $? -ne 0 ]; then
    print_error "Failed to deploy CloudFormation stack"
    exit 1
fi

# Wait for stack operation to complete
print_status "Waiting for stack operation to complete..."
aws cloudformation wait "$WAIT_CONDITION" --stack-name "$STACK_NAME" --region "$REGION"

if [ $? -ne 0 ]; then
    print_error "Stack operation failed or timed out"
    print_error "Check the AWS CloudFormation console for details"
    exit 1
fi

# Get stack outputs
print_status "Retrieving stack outputs..."
OUTPUTS=$(aws cloudformation describe-stacks \
    --stack-name "$STACK_NAME" \
    --region "$REGION" \
    --query 'Stacks[0].Outputs' \
    --output table)

if [ $? -ne 0 ]; then
    print_error "Failed to retrieve stack outputs"
    exit 1
fi

# Get the Omni URL specifically
OMNI_URL=$(aws cloudformation describe-stacks \
    --stack-name "$STACK_NAME" \
    --region "$REGION" \
    --query 'Stacks[0].Outputs[?OutputKey==`OmniURL`].OutputValue' \
    --output text)

print_status "Deployment completed successfully!"
echo ""
echo "=============================================="
echo "  OMNI DEPLOYMENT COMPLETE"
echo "=============================================="
echo ""
echo "Customer: $CUSTOMER_NAME"
echo "Account: $CUSTOMER_ACCOUNT_ID"
echo "Region: $REGION"
echo "Stack: $STACK_NAME"
echo ""
echo "Omni URL: $OMNI_URL"
echo ""
echo "All stack outputs:"
echo "$OUTPUTS"
echo ""
echo "=============================================="
echo "  NEXT STEPS"
echo "=============================================="
echo ""
echo "1. Test the application: $OMNI_URL"
echo "2. Share the URL with the customer"
echo "3. Provide admin credentials (if configured)"
echo "4. Monitor CloudWatch logs: /ecs/omni-$CUSTOMER_NAME"
echo ""
print_status "Deployment script completed!"
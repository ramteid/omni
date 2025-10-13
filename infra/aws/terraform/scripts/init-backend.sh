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

print_status "Initializing Terraform S3 backend..."

# Get AWS account ID
ACCOUNT_ID=$(aws sts get-caller-identity --query Account --output text)
if [ $? -ne 0 ]; then
    print_error "Failed to get AWS account ID. Check your AWS credentials."
    exit 1
fi

print_status "AWS Account ID: $ACCOUNT_ID"

# Set variables
BUCKET_NAME="omni-terraform-state-${ACCOUNT_ID}"
REGION="${AWS_DEFAULT_REGION:-us-east-1}"
DYNAMODB_TABLE="omni-terraform-locks"

print_status "Bucket name: $BUCKET_NAME"
print_status "Region: $REGION"
print_status "DynamoDB table: $DYNAMODB_TABLE"

# Create S3 bucket
print_status "Creating S3 bucket..."
if aws s3api head-bucket --bucket "$BUCKET_NAME" 2>/dev/null; then
    print_warning "S3 bucket already exists: $BUCKET_NAME"
else
    if [ "$REGION" = "us-east-1" ]; then
        aws s3api create-bucket --bucket "$BUCKET_NAME" --region "$REGION"
    else
        aws s3api create-bucket --bucket "$BUCKET_NAME" --region "$REGION" \
            --create-bucket-configuration LocationConstraint="$REGION"
    fi
    print_status "S3 bucket created: $BUCKET_NAME"
fi

# Enable versioning
print_status "Enabling versioning..."
aws s3api put-bucket-versioning --bucket "$BUCKET_NAME" \
    --versioning-configuration Status=Enabled

# Enable encryption
print_status "Enabling encryption..."
aws s3api put-bucket-encryption --bucket "$BUCKET_NAME" \
    --server-side-encryption-configuration '{
        "Rules": [{
            "ApplyServerSideEncryptionByDefault": {
                "SSEAlgorithm": "AES256"
            },
            "BucketKeyEnabled": true
        }]
    }'

# Block public access
print_status "Blocking public access..."
aws s3api put-public-access-block --bucket "$BUCKET_NAME" \
    --public-access-block-configuration \
    BlockPublicAcls=true,IgnorePublicAcls=true,BlockPublicPolicy=true,RestrictPublicBuckets=true

# Add lifecycle policy
print_status "Adding lifecycle policy..."
aws s3api put-bucket-lifecycle-configuration --bucket "$BUCKET_NAME" \
    --lifecycle-configuration '{
        "Rules": [{
            "Id": "DeleteOldVersions",
            "Status": "Enabled",
            "NoncurrentVersionExpiration": {
                "NoncurrentDays": 90
            }
        }]
    }'

# Create DynamoDB table
print_status "Creating DynamoDB table..."
if aws dynamodb describe-table --table-name "$DYNAMODB_TABLE" --region "$REGION" 2>/dev/null >/dev/null; then
    print_warning "DynamoDB table already exists: $DYNAMODB_TABLE"
else
    aws dynamodb create-table \
        --table-name "$DYNAMODB_TABLE" \
        --attribute-definitions AttributeName=LockID,AttributeType=S \
        --key-schema AttributeName=LockID,KeyType=HASH \
        --billing-mode PAY_PER_REQUEST \
        --region "$REGION" \
        --tags Key=Application,Value=Omni Key=ManagedBy,Value=Terraform

    print_status "Waiting for DynamoDB table to be active..."
    aws dynamodb wait table-exists --table-name "$DYNAMODB_TABLE" --region "$REGION"
    print_status "DynamoDB table created: $DYNAMODB_TABLE"
fi

print_status ""
print_status "=============================================="
print_status "  TERRAFORM BACKEND INITIALIZATION COMPLETE"
print_status "=============================================="
print_status ""
print_status "S3 Bucket: $BUCKET_NAME"
print_status "DynamoDB Table: $DYNAMODB_TABLE"
print_status "Region: $REGION"
print_status ""
print_status "Next steps:"
print_status "1. Copy backend configuration:"
print_status "   cp backend.tf.example backend.tf"
print_status ""
print_status "2. Update backend.tf with your account ID:"
print_status "   sed -i 's/{account-id}/$ACCOUNT_ID/g' backend.tf"
print_status ""
print_status "3. Initialize Terraform:"
print_status "   terraform init"
print_status ""

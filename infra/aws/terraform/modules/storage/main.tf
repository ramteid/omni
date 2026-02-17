# S3 buckets for Omni storage

# Content storage bucket - stores document content
resource "aws_s3_bucket" "content" {
  bucket = "omni-${var.customer_name}-content"

  tags = merge(var.tags, {
    Name    = "omni-${var.customer_name}-content"
    Purpose = "Document content storage"
  })
}

resource "aws_s3_bucket_versioning" "content" {
  bucket = aws_s3_bucket.content.id

  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "content" {
  bucket = aws_s3_bucket.content.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

resource "aws_s3_bucket_public_access_block" "content" {
  bucket = aws_s3_bucket.content.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

# Batch inference bucket - stores batch job input/output files
resource "aws_s3_bucket" "batch" {
  bucket = "omni-${var.customer_name}-batch-inference"

  tags = merge(var.tags, {
    Name    = "omni-${var.customer_name}-batch-inference"
    Purpose = "Bedrock batch inference"
  })
}

resource "aws_s3_bucket_server_side_encryption_configuration" "batch" {
  bucket = aws_s3_bucket.batch.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

resource "aws_s3_bucket_public_access_block" "batch" {
  bucket = aws_s3_bucket.batch.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

# Lifecycle policy to clean up old batch files
resource "aws_s3_bucket_lifecycle_configuration" "batch" {
  bucket = aws_s3_bucket.batch.id

  rule {
    id     = "delete-old-batch-files"
    status = "Enabled"

    filter {
      prefix = "" # Apply to all objects
    }

    expiration {
      days = 7 # Delete batch files after 7 days
    }
  }
}

# IAM role for Bedrock batch inference
# This role is assumed by Bedrock service to read/write S3 files
resource "aws_iam_role" "bedrock_batch" {
  name = "omni-${var.customer_name}-bedrock-batch-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect = "Allow"
      Principal = {
        Service = "bedrock.amazonaws.com"
      }
      Action = "sts:AssumeRole"
      Condition = {
        StringEquals = {
          "aws:SourceAccount" = data.aws_caller_identity.current.account_id
        }
      }
    }]
  })

  inline_policy {
    name = "BedrockBatchS3Access"
    policy = jsonencode({
      Version = "2012-10-17"
      Statement = [
        {
          Effect = "Allow"
          Action = [
            "s3:GetObject",
            "s3:PutObject",
            "s3:ListBucket"
          ]
          Resource = [
            aws_s3_bucket.batch.arn,
            "${aws_s3_bucket.batch.arn}/*"
          ]
        }
      ]
    })
  }

  inline_policy {
    name = "BedrockModelInvoke"
    policy = jsonencode({
      Version = "2012-10-17"
      Statement = [{
        Effect = "Allow"
        Action = [
          "bedrock:InvokeModel"
        ]
        Resource = [
          "arn:aws:bedrock:*:*:inference-profile/us.anthropic.*",
          "arn:aws:bedrock:*:*:inference-profile/eu.anthropic.*",
          "arn:aws:bedrock:*::foundation-model/anthropic.*",
          "arn:aws:bedrock:*:*:inference-profile/us.amazon.*",
          "arn:aws:bedrock:*:*:inference-profile/eu.amazon.*",
          "arn:aws:bedrock:*::foundation-model/amazon.*",
          "arn:aws:bedrock:*::foundation-model/amazon.titan-embed-text-*"
        ]
      }]
    })
  }

  tags = merge(var.tags, {
    Name    = "omni-${var.customer_name}-bedrock-batch-role"
    Purpose = "Bedrock batch inference S3 access"
  })
}

data "aws_caller_identity" "current" {}

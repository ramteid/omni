output "content_bucket_name" {
  description = "Name of the S3 bucket for content storage"
  value       = aws_s3_bucket.content.id
}

output "content_bucket_arn" {
  description = "ARN of the S3 bucket for content storage"
  value       = aws_s3_bucket.content.arn
}

output "batch_bucket_name" {
  description = "Name of the S3 bucket for batch inference"
  value       = aws_s3_bucket.batch.id
}

output "batch_bucket_arn" {
  description = "ARN of the S3 bucket for batch inference"
  value       = aws_s3_bucket.batch.arn
}

output "bedrock_batch_role_arn" {
  description = "ARN of the IAM role for Bedrock batch inference"
  value       = aws_iam_role.bedrock_batch.arn
}

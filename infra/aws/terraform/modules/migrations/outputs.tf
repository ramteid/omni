output "lambda_function_arn" {
  description = "Lambda function ARN"
  value       = aws_lambda_function.migrator.arn
}

output "lambda_function_name" {
  description = "Lambda function name"
  value       = aws_lambda_function.migrator.function_name
}

output "migration_completed" {
  description = "Migration completion status"
  value       = null_resource.run_migrations.id != "" ? "completed" : "pending"
}

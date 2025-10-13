output "endpoint" {
  description = "Database endpoint address"
  value       = aws_db_instance.postgresql.address
}

output "port" {
  description = "Database port"
  value       = aws_db_instance.postgresql.port
}

output "database_name" {
  description = "Database name"
  value       = aws_db_instance.postgresql.db_name
}

output "instance_id" {
  description = "Database instance ID"
  value       = aws_db_instance.postgresql.id
}

output "arn" {
  description = "Database ARN"
  value       = aws_db_instance.postgresql.arn
}

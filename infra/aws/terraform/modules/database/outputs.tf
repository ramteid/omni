output "endpoint" {
  description = "ParadeDB endpoint address"
  value       = "paradedb.omni-${var.customer_name}.local"
}

output "port" {
  description = "Database port"
  value       = 5432
}

output "database_name" {
  description = "Database name"
  value       = var.database_name
}

output "paradedb_capacity_provider_name" {
  description = "ParadeDB capacity provider name"
  value       = aws_ecs_capacity_provider.paradedb.name
}

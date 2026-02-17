output "cluster_name" {
  description = "ECS cluster name"
  value       = var.cluster_name
}

output "cluster_arn" {
  description = "ECS cluster ARN"
  value       = var.cluster_arn
}

output "web_service_name" {
  description = "Web service name"
  value       = aws_ecs_service.web.name
}

output "searcher_service_name" {
  description = "Searcher service name"
  value       = aws_ecs_service.searcher.name
}

output "indexer_service_name" {
  description = "Indexer service name"
  value       = aws_ecs_service.indexer.name
}

output "ai_service_name" {
  description = "AI service name"
  value       = aws_ecs_service.ai.name
}

output "google_connector_service_name" {
  description = "Google connector service name"
  value       = aws_ecs_service.google_connector.name
}

output "migrator_task_definition_arn" {
  description = "Migrator task definition ARN"
  value       = aws_ecs_task_definition.migrator.arn
}

output "task_execution_role_arn" {
  description = "ECS task execution role ARN"
  value       = aws_iam_role.ecs_task_execution.arn
}

output "task_role_arn" {
  description = "ECS task role ARN"
  value       = aws_iam_role.ecs_task.arn
}

output "service_discovery_namespace_id" {
  description = "Service discovery namespace ID"
  value       = var.service_discovery_namespace_id
}

output "connector_manager_service_name" {
  description = "Connector manager service name"
  value       = aws_ecs_service.connector_manager.name
}

output "slack_connector_service_name" {
  description = "Slack connector service name"
  value       = aws_ecs_service.slack_connector.name
}

output "atlassian_connector_service_name" {
  description = "Atlassian connector service name"
  value       = aws_ecs_service.atlassian_connector.name
}

output "web_connector_service_name" {
  description = "Web connector service name"
  value       = aws_ecs_service.web_connector.name
}

output "github_connector_service_name" {
  description = "GitHub connector service name"
  value       = aws_ecs_service.github_connector.name
}

output "hubspot_connector_service_name" {
  description = "HubSpot connector service name"
  value       = aws_ecs_service.hubspot_connector.name
}

output "microsoft_connector_service_name" {
  description = "Microsoft connector service name"
  value       = aws_ecs_service.microsoft_connector.name
}

output "notion_connector_service_name" {
  description = "Notion connector service name"
  value       = aws_ecs_service.notion_connector.name
}

output "fireflies_connector_service_name" {
  description = "Fireflies connector service name"
  value       = aws_ecs_service.fireflies_connector.name
}

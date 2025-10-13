variable "customer_name" {
  description = "Customer name for resource naming"
  type        = string
}

variable "environment" {
  description = "Environment (production, staging, development)"
  type        = string
  default     = "production"
}

variable "cluster_name" {
  description = "ECS cluster name"
  type        = string
}

variable "migrator_task_definition_arn" {
  description = "Migrator task definition ARN"
  type        = string
}

variable "subnet_ids" {
  description = "List of subnet IDs for migration task"
  type        = list(string)
}

variable "security_group_id" {
  description = "Security group ID for migration task"
  type        = string
}

variable "region" {
  description = "AWS region"
  type        = string
}

variable "task_execution_role_arn" {
  description = "ECS task execution role ARN"
  type        = string
}

variable "task_role_arn" {
  description = "ECS task role ARN"
  type        = string
}

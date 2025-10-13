variable "customer_name" {
  description = "Customer name for resource naming"
  type        = string
}

variable "environment" {
  description = "Environment (production, staging, development)"
  type        = string
  default     = "production"
}

variable "node_type" {
  description = "ElastiCache node type"
  type        = string
  default     = "cache.t3.micro"
}

variable "engine_version" {
  description = "Redis engine version"
  type        = string
  default     = "7.1"
}

variable "subnet_ids" {
  description = "List of subnet IDs for cache subnet group"
  type        = list(string)
}

variable "security_group_id" {
  description = "Security group ID for Redis cluster"
  type        = string
}

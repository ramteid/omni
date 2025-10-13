locals {
  common_tags = {
    Application = "Omni"
    Customer    = var.customer_name
    Environment = var.environment
    ManagedBy   = "Terraform"
  }
}

resource "aws_elasticache_subnet_group" "main" {
  name        = "omni-${var.customer_name}-redis-subnet-group"
  description = "Subnet group for Redis"
  subnet_ids  = var.subnet_ids

  tags = merge(local.common_tags, {
    Name = "omni-${var.customer_name}-redis-subnet-group"
  })
}

resource "aws_elasticache_cluster" "redis" {
  cluster_id = "omni-${var.customer_name}-redis"

  engine               = "redis"
  engine_version       = var.engine_version
  node_type            = var.node_type
  num_cache_nodes      = 1
  parameter_group_name = "default.redis7"
  port                 = 6379

  subnet_group_name  = aws_elasticache_subnet_group.main.name
  security_group_ids = [var.security_group_id]

  tags = merge(local.common_tags, {
    Name = "omni-${var.customer_name}-redis"
  })
}

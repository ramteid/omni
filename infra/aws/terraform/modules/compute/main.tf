locals {
  common_tags = {
    Application = "Omni"
    Customer    = var.customer_name
    Environment = var.environment
    ManagedBy   = "Terraform"
  }

  app_url = "https://${var.custom_domain}"
}

data "aws_caller_identity" "current" {}

resource "aws_ecs_cluster" "main" {
  name = "omni-${var.customer_name}-cluster"

  setting {
    name  = "containerInsights"
    value = "enabled"
  }

  tags = merge(local.common_tags, {
    Name = "omni-${var.customer_name}-cluster"
  })
}

resource "aws_service_discovery_private_dns_namespace" "main" {
  name        = "omni-${var.customer_name}.local"
  description = "Private DNS namespace for Omni services"
  vpc         = var.vpc_id

  tags = local.common_tags
}

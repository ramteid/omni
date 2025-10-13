locals {
  common_tags = {
    Application = "Omni"
    Customer    = var.customer_name
    Environment = var.environment
    ManagedBy   = "Terraform"
  }
}

resource "aws_cloudwatch_log_group" "ecs" {
  name              = "/ecs/omni-${var.customer_name}"
  retention_in_days = var.log_retention_days

  tags = merge(local.common_tags, {
    Name = "/ecs/omni-${var.customer_name}"
  })
}

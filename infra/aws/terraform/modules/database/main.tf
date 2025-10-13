locals {
  common_tags = {
    Application = "Omni"
    Customer    = var.customer_name
    Environment = var.environment
    ManagedBy   = "Terraform"
  }
}

resource "aws_db_subnet_group" "main" {
  name        = "omni-${var.customer_name}-db-subnet-group"
  description = "Subnet group for RDS database"
  subnet_ids  = var.subnet_ids

  tags = merge(local.common_tags, {
    Name = "omni-${var.customer_name}-db-subnet-group"
  })
}

resource "aws_db_instance" "postgresql" {
  identifier     = "omni-${var.customer_name}-postgres"
  engine         = "postgres"
  engine_version = "17.4"

  instance_class    = var.instance_class
  allocated_storage = var.allocated_storage
  storage_type      = "gp3"
  storage_encrypted = true

  db_name  = var.database_name
  username = var.database_username
  password = var.database_password

  db_subnet_group_name   = aws_db_subnet_group.main.name
  vpc_security_group_ids = [var.security_group_id]

  backup_retention_period = var.backup_retention_period
  backup_window           = "03:00-04:00"
  maintenance_window      = "sun:04:00-sun:05:00"

  multi_az               = var.multi_az
  deletion_protection    = var.deletion_protection
  skip_final_snapshot    = var.skip_final_snapshot
  final_snapshot_identifier = var.skip_final_snapshot ? null : "omni-${var.customer_name}-postgres-final-${formatdate("YYYY-MM-DD-hhmm", timestamp())}"

  enabled_cloudwatch_logs_exports = ["postgresql", "upgrade"]

  ca_cert_identifier = "rds-ca-rsa2048-g1"

  tags = merge(local.common_tags, {
    Name = "omni-${var.customer_name}-postgres"
  })
}

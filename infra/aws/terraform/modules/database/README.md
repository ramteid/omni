# Database Module

This module creates an RDS PostgreSQL instance with pgvector support for Omni.

## Resources Created

- RDS DB Subnet Group
- RDS PostgreSQL 17.2 Instance
  - pgvector extension support
  - Encrypted storage (gp3)
  - Automatic backups
  - CloudWatch Logs integration

## Usage

```hcl
module "database" {
  source = "./modules/database"

  customer_name     = "acme-corp"
  environment       = "production"
  instance_class    = "db.t3.micro"
  database_name     = "omni"
  database_username = "omni"
  database_password = module.secrets.database_password

  subnet_ids        = module.networking.private_subnet_ids
  security_group_id = module.networking.database_security_group_id

  skip_final_snapshot = false
}
```

## Inputs

| Name | Description | Type | Default | Required |
|------|-------------|------|---------|----------|
| customer_name | Customer name for resource naming | string | - | yes |
| environment | Environment (production, staging, development) | string | "production" | no |
| instance_class | RDS instance type | string | "db.t3.micro" | no |
| allocated_storage | Allocated storage in GB | number | 20 | no |
| database_name | PostgreSQL database name | string | "omni" | no |
| database_username | PostgreSQL master username | string | "omni" | no |
| database_password | PostgreSQL master password | string | - | yes |
| subnet_ids | List of subnet IDs for DB subnet group | list(string) | - | yes |
| security_group_id | Security group ID for database | string | - | yes |
| backup_retention_period | Backup retention period in days | number | 7 | no |
| multi_az | Enable Multi-AZ deployment | bool | false | no |
| deletion_protection | Enable deletion protection | bool | false | no |
| skip_final_snapshot | Skip final DB snapshot on deletion | bool | false | no |

## Outputs

| Name | Description |
|------|-------------|
| endpoint | Database endpoint address |
| port | Database port |
| database_name | Database name |
| instance_id | Database instance ID |
| arn | Database ARN |

## PostgreSQL Configuration

### Engine Version
- PostgreSQL 17.2

### Storage
- Type: gp3 (General Purpose SSD)
- Default size: 20 GB
- Encrypted at rest

### Backups
- Retention: 7 days (configurable)
- Backup window: 03:00-04:00 UTC
- Maintenance window: Sunday 04:00-05:00 UTC

### High Availability
- Single-AZ by default (cost optimization)
- Multi-AZ can be enabled for production workloads

### Monitoring
- CloudWatch Logs: postgresql, upgrade
- Enhanced monitoring available

### Security
- SSL/TLS required for connections
- Encryption at rest enabled
- Private subnet deployment only
- Security group restricts access to ECS services

## pgvector Extension

After database creation, connect and enable pgvector:

```sql
CREATE EXTENSION IF NOT EXISTS vector;
```

This is handled automatically by the migrations module.

## Cost Optimization

### Development/Staging
```hcl
instance_class      = "db.t3.micro"
multi_az            = false
skip_final_snapshot = true
```

### Production
```hcl
instance_class      = "db.t3.small"  # or larger
multi_az            = true
skip_final_snapshot = false
deletion_protection = true
```

## Scaling

To resize the database:

1. Update `instance_class` or `allocated_storage`
2. Run `terraform plan` to preview changes
3. Run `terraform apply`
4. Terraform will schedule the modification during the maintenance window

## Final Snapshots

When `skip_final_snapshot = false`, a final snapshot is created on deletion with a timestamp-based identifier. This snapshot can be used to restore the database if needed.

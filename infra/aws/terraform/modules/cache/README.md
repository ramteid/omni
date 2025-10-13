# Cache Module

This module creates an ElastiCache Redis cluster for Omni.

## Resources Created

- ElastiCache Subnet Group
- ElastiCache Redis Cluster (single node)

## Usage

```hcl
module "cache" {
  source = "./modules/cache"

  customer_name     = "acme-corp"
  environment       = "production"
  node_type         = "cache.t3.micro"
  engine_version    = "7.1"

  subnet_ids        = module.networking.private_subnet_ids
  security_group_id = module.networking.redis_security_group_id
}
```

## Inputs

| Name | Description | Type | Default | Required |
|------|-------------|------|---------|----------|
| customer_name | Customer name for resource naming | string | - | yes |
| environment | Environment (production, staging, development) | string | "production" | no |
| node_type | ElastiCache node type | string | "cache.t3.micro" | no |
| engine_version | Redis engine version | string | "7.1" | no |
| subnet_ids | List of subnet IDs for cache subnet group | list(string) | - | yes |
| security_group_id | Security group ID for Redis cluster | string | - | yes |

## Outputs

| Name | Description |
|------|-------------|
| endpoint | Redis cluster endpoint address |
| port | Redis cluster port |
| cluster_id | Redis cluster ID |
| arn | Redis cluster ARN |

## Redis Configuration

### Engine
- Redis 7.1 (latest stable)
- Default parameter group

### Deployment
- Single node (cost optimization)
- Can be upgraded to Multi-AZ for production

### Network
- Private subnet deployment
- Security group restricts access to ECS services

## Use Cases

Omni uses Redis for:
- Session storage
- Search result caching
- Message queue (connector events)
- Temporary data storage

## Cost Optimization

### Development/Staging
```hcl
node_type = "cache.t3.micro"
```

### Production
```hcl
node_type = "cache.t3.small"  # or larger based on load
```

## Scaling

To resize the Redis cluster:

1. Update `node_type`
2. Run `terraform plan` to preview changes
3. Run `terraform apply`
4. Terraform will apply the change with minimal downtime

## High Availability

For production workloads requiring high availability, consider:
- Using `aws_elasticache_replication_group` instead
- Enabling automatic failover
- Deploying across multiple AZs

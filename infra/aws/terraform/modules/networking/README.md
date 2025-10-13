# Networking Module

This module creates the VPC, subnets, routing, and security groups for Omni infrastructure.

## Resources Created

- VPC (10.0.0.0/16)
- Internet Gateway
- NAT Gateway with Elastic IP
- 2 Public Subnets (10.0.1.0/24, 10.0.2.0/24)
- 2 Private Subnets (10.0.11.0/24, 10.0.12.0/24)
- Public and Private Route Tables
- 4 Security Groups:
  - ALB Security Group (ports 80, 443)
  - ECS Security Group (ports 3000-3004)
  - Database Security Group (port 5432)
  - Redis Security Group (port 6379)

## Usage

```hcl
module "networking" {
  source = "./modules/networking"

  customer_name = "acme-corp"
  environment   = "production"
  vpc_cidr      = "10.0.0.0/16"
}
```

## Inputs

| Name | Description | Type | Default | Required |
|------|-------------|------|---------|----------|
| customer_name | Customer name for resource naming | string | - | yes |
| environment | Environment (production, staging, development) | string | "production" | no |
| vpc_cidr | CIDR block for VPC | string | "10.0.0.0/16" | no |

## Outputs

| Name | Description |
|------|-------------|
| vpc_id | ID of the VPC |
| public_subnet_ids | IDs of public subnets |
| private_subnet_ids | IDs of private subnets |
| alb_security_group_id | ID of ALB security group |
| ecs_security_group_id | ID of ECS security group |
| database_security_group_id | ID of database security group |
| redis_security_group_id | ID of Redis security group |
| nat_gateway_id | ID of NAT Gateway |

## Network Architecture

```
Internet
    |
    v
Internet Gateway
    |
    v
Public Subnets (2 AZs)
    |
    +-- Application Load Balancer
    +-- NAT Gateway
    |
    v
Private Subnets (2 AZs)
    |
    +-- ECS Services
    +-- RDS Database
    +-- ElastiCache Redis
```

## Security Groups

### ALB Security Group
- Ingress: 80 (HTTP), 443 (HTTPS) from Internet
- Egress: All traffic

### ECS Security Group
- Ingress: 3000 from ALB (web service)
- Ingress: 3001-3004 from self (inter-service)
- Egress: All traffic

### Database Security Group
- Ingress: 5432 from ECS
- Egress: All traffic

### Redis Security Group
- Ingress: 6379 from ECS
- Egress: All traffic

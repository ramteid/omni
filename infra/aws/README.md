# Omni AWS Deployment

This directory contains the infrastructure and deployment scripts for deploying Omni into customer AWS accounts using the managed service approach.

## Overview

Omni uses a dual-pronged deployment strategy:
- **Self-hosted**: Customers deploy and manage Omni themselves
- **Managed service**: Customers run a bootstrap template, Omni team handles deployment

This directory supports the **managed service** approach.

## Directory Structure

```
infra/aws/
├── cloudformation/
│   ├── main.yaml                    # Customer bootstrap template
│   └── omni-infrastructure.yaml     # Full infrastructure deployment
├── scripts/
│   └── deploy-customer.sh           # Deployment automation script
├── parameters/
│   └── default.json                 # Default parameter values
└── README.md                        # This file
```

## Customer Onboarding Process

### Step 1: Customer Runs Bootstrap Template

**Customer Instructions:**

1. Download the bootstrap template: `infra/aws/cloudformation/main.yaml`
2. Deploy via AWS CloudFormation Console or CLI:
   ```bash
   aws cloudformation create-stack \
     --stack-name omni-bootstrap \
     --template-body file://main.yaml \
     --capabilities CAPABILITY_NAMED_IAM \
     --parameters ParameterKey=ExternalId,ParameterValue=<secure-external-id>
   ```
3. Provide Omni team with:
   - AWS Account ID (from stack outputs)
   - AWS Region (from stack outputs)
   - External ID (used in step 2)

**What the bootstrap template creates:**
- Cross-account IAM role for Omni team deployment access
- S3 bucket for deployment artifacts
- ECS task execution role
- Required permissions for infrastructure deployment

### Step 2: Omni Team Deployment

**Prerequisites:**
- Customer has completed Step 1
- Container images are published to GitHub Container Registry (public)
- AWS CLI configured with appropriate permissions

**Deployment Command:**
```bash
./infra/aws/scripts/deploy-customer.sh <account-id> <region> <external-id> [customer-name] [github-org]
```

**Example:**
```bash
./infra/aws/scripts/deploy-customer.sh 123456789012 us-east-1 abc123xyz789 acme-corp mycompany
```

**What happens during deployment:**
1. Script assumes customer's deployment role
2. Deploys complete infrastructure stack:
   - VPC with public/private subnets
   - ECS cluster and services
   - RDS PostgreSQL with pgvector
   - ElastiCache Redis
   - Application Load Balancer
   - CloudWatch logging
3. Outputs customer's Omni URL and connection details

## Infrastructure Components

### Networking
- **VPC**: Isolated network (10.0.0.0/16)
- **Public Subnets**: For load balancer (10.0.1.0/24, 10.0.2.0/24)
- **Private Subnets**: For applications and databases (10.0.11.0/24, 10.0.12.0/24)
- **NAT Gateway**: Outbound internet access for private subnets
- **Security Groups**: Layered security for each tier

### Compute
- **ECS Fargate**: Serverless container orchestration
- **Auto Scaling**: Automatic capacity management
- **Load Balancer**: HTTP/HTTPS traffic distribution
- **Container Images**: Pulled from GitHub Container Registry

### Data Storage
- **RDS PostgreSQL**: Primary database with pgvector extension
- **ElastiCache Redis**: Caching and session storage
- **Secrets Manager**: Secure credential storage
- **CloudWatch Logs**: Application and infrastructure logs

### Services Deployed
- **omni-web**: SvelteKit frontend and API (port 3000)
- **omni-searcher**: Search service (port 8001)
- **omni-indexer**: Document processing service (port 8002)  
- **omni-ai**: AI and embeddings service (port 8003)

## Configuration

### Default Resource Sizes
- **Database**: db.t3.micro (suitable for small workloads)
- **Redis**: cache.t3.micro  
- **ECS Tasks**: 512 CPU / 1024 MB memory

### Customization
Modify `infra/aws/parameters/default.json` to change default resource sizes:
```json
[
  {
    "ParameterKey": "DBInstanceClass",
    "ParameterValue": "db.t3.small"
  },
  {
    "ParameterKey": "ECSTaskCpu", 
    "ParameterValue": "1024"
  }
]
```

Or pass parameters directly to deployment script:
```bash
# Coming soon: parameter override support
./deploy-customer.sh 123456789012 us-east-1 abc123xyz789 acme-corp mycompany --db-instance db.t3.small
```

## Container Images

Images are pulled from GitHub Container Registry:
- `ghcr.io/{github-org}/omni-web:latest`
- `ghcr.io/{github-org}/omni-searcher:latest`  
- `ghcr.io/{github-org}/omni-indexer:latest`
- `ghcr.io/{github-org}/omni-ai:latest`

**Requirements:**
- Images must be public (no authentication required)
- Images should be tagged as `latest` for stable releases
- All services must be containerized and include health check endpoints

## Security

### Network Security
- Private subnets for all application and database components
- Public subnets only for load balancer
- Security groups restrict traffic between tiers
- NAT Gateway for secure outbound internet access

### Access Control
- Cross-account IAM role with least-privilege permissions
- External ID requirement for role assumption
- Secrets Manager for sensitive data
- No hardcoded credentials in templates

### Data Protection
- RDS encryption at rest
- VPC isolation
- TLS termination at load balancer
- CloudWatch Logs for audit trails

## Monitoring and Logging

### CloudWatch Integration
- **Log Groups**: `/ecs/omni-{customer-name}`
- **Log Streams**: Separate streams per service (web, searcher, indexer, ai)
- **Retention**: 30 days default
- **Metrics**: ECS service metrics, RDS performance, ALB request metrics

### Health Checks
- **ALB Health Checks**: HTTP `/health` endpoint on omni-web
- **ECS Health Checks**: Container-level health monitoring
- **RDS Monitoring**: Database performance metrics

## Troubleshooting

### Common Issues

**Deployment fails with "Role not found"**
- Verify customer has run bootstrap template successfully
- Check External ID matches what customer used
- Confirm Account ID is correct

**Services won't start**
- Check CloudWatch logs: `/ecs/omni-{customer-name}`
- Verify container images are public and accessible
- Check database connectivity and credentials

**Application not accessible**
- Verify security groups allow ALB → ECS traffic
- Check ECS service is running and healthy
- Confirm load balancer target group health

**Database connection issues**
- Verify RDS instance is running
- Check security groups allow ECS → RDS traffic  
- Confirm database credentials in Secrets Manager

### Debugging Commands

```bash
# Check stack status
aws cloudformation describe-stacks --stack-name omni-{customer}-infrastructure

# View ECS service status  
aws ecs describe-services --cluster omni-{customer}-cluster --services omni-{customer}-service

# Check container logs
aws logs get-log-events --log-group-name /ecs/omni-{customer} --log-stream-name web/omni-web/{task-id}

# Test database connectivity
aws rds describe-db-instances --db-instance-identifier omni-{customer}-postgres
```

## Cost Optimization

### Resource Sizing
- Start with t3.micro instances for development/small workloads
- Monitor CloudWatch metrics and scale as needed
- Use RDS performance insights for database optimization

### Cost Monitoring
- All resources tagged with customer information
- Use AWS Cost Explorer with tag-based filtering
- Set up billing alerts for customer accounts

## Updates and Maintenance

### Application Updates
1. Build and publish new container images to GHCR
2. Update ECS services to pull latest images:
   ```bash
   aws ecs update-service --cluster omni-{customer}-cluster --service omni-{customer}-service --force-new-deployment
   ```

### Infrastructure Updates
1. Modify CloudFormation templates
2. Re-run deployment script (will perform stack update)
3. Monitor CloudFormation events for successful completion

## Support

For deployment issues:
1. Check CloudWatch logs for application errors
2. Review CloudFormation events for infrastructure issues  
3. Verify GitHub Container Registry image availability
4. Contact Omni support with customer account details and error messages
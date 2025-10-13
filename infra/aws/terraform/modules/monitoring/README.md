# Monitoring Module

This module creates CloudWatch Log Group for Omni ECS services.

## Resources Created

- CloudWatch Log Group `/ecs/omni-{customer-name}`

## Usage

```hcl
module "monitoring" {
  source = "./modules/monitoring"

  customer_name      = "acme-corp"
  environment        = "production"
  log_retention_days = 30
}
```

## Inputs

| Name | Description | Type | Default | Required |
|------|-------------|------|---------|----------|
| customer_name | Customer name for resource naming | string | - | yes |
| environment | Environment (production, staging, development) | string | "production" | no |
| log_retention_days | CloudWatch Logs retention in days | number | 30 | no |

## Outputs

| Name | Description |
|------|-------------|
| log_group_name | CloudWatch Log Group name |
| log_group_arn | CloudWatch Log Group ARN |

## Log Organization

All ECS services write to the same log group with different stream prefixes:

- `/ecs/omni-{customer}/web/...`
- `/ecs/omni-{customer}/searcher/...`
- `/ecs/omni-{customer}/indexer/...`
- `/ecs/omni-{customer}/ai/...`
- `/ecs/omni-{customer}/google-connector/...`
- `/ecs/omni-{customer}/migrator/...`

## Retention Policy

Default retention: 30 days

Adjust based on compliance requirements:
- Development: 7 days
- Staging: 14 days
- Production: 30-90 days

## Cost Optimization

CloudWatch Logs pricing is based on:
- Data ingestion (per GB)
- Storage (per GB-month)
- Data transfer

To reduce costs:
- Set appropriate retention periods
- Filter verbose log levels in production
- Archive old logs to S3 if needed

## Viewing Logs

Using AWS CLI:
```bash
# List log streams
aws logs describe-log-streams \
  --log-group-name /ecs/omni-{customer} \
  --order-by LastEventTime \
  --descending

# Tail logs for web service
aws logs tail /ecs/omni-{customer} \
  --follow \
  --filter-pattern "web"
```

Using AWS Console:
1. Navigate to CloudWatch > Logs > Log groups
2. Search for `/ecs/omni-{customer}`
3. Click to view log streams
4. Use CloudWatch Logs Insights for advanced queries

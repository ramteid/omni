# Migrations Module

This module handles database migrations for Omni by running a one-time ECS task via Lambda.

## Resources Created

- Lambda IAM Role
- Lambda Function (Python)
- Null Resource (triggers migration)

## How It Works

1. Lambda function is created with migration logic
2. Null resource triggers Lambda invocation
3. Lambda runs ECS Fargate task (migrator)
4. Lambda waits for task completion
5. Returns success/failure status

## Usage

```hcl
module "migrations" {
  source = "./modules/migrations"

  customer_name = "acme-corp"
  environment   = "production"

  cluster_name                 = module.compute.cluster_name
  migrator_task_definition_arn = module.compute.migrator_task_definition_arn
  subnet_ids                   = module.networking.private_subnet_ids
  security_group_id            = module.networking.ecs_security_group_id
  region                       = var.region

  task_execution_role_arn = module.compute.task_execution_role_arn
  task_role_arn           = module.compute.task_role_arn
}
```

## Inputs

| Name | Description | Type | Default | Required |
|------|-------------|------|---------|----------|
| customer_name | Customer name for resource naming | string | - | yes |
| environment | Environment | string | "production" | no |
| cluster_name | ECS cluster name | string | - | yes |
| migrator_task_definition_arn | Migrator task definition ARN | string | - | yes |
| subnet_ids | Subnet IDs for migration task | list(string) | - | yes |
| security_group_id | Security group ID for migration task | string | - | yes |
| region | AWS region | string | - | yes |
| task_execution_role_arn | Task execution role ARN | string | - | yes |
| task_role_arn | Task role ARN | string | - | yes |

## Outputs

| Name | Description |
|------|-------------|
| lambda_function_arn | Lambda function ARN |
| lambda_function_name | Lambda function name |
| migration_completed | Migration completion status |

## Migration Process

### 1. Lambda Invocation
The null_resource triggers Lambda with:
- Cluster name
- Task definition ARN
- Network configuration

### 2. Task Execution
Lambda runs the migrator ECS task:
```python
ecs.run_task(
    cluster=cluster,
    taskDefinition=task_definition,
    launchType='FARGATE',
    networkConfiguration={...}
)
```

### 3. Wait for Completion
Lambda waits up to 10 minutes for task to complete.

### 4. Exit Code Check
Lambda checks container exit code:
- 0 = Success
- Non-zero = Failure

## Manual Migration Run

To manually run migrations:

```bash
aws lambda invoke \
  --function-name omni-{customer}-migrator \
  --payload '{"Cluster":"omni-{customer}-cluster","TaskDefinition":"...","Subnets":"...","SecurityGroups":"..."}' \
  response.json

cat response.json
```

Or directly run ECS task:

```bash
aws ecs run-task \
  --cluster omni-{customer}-cluster \
  --task-definition omni-{customer}-migrator \
  --launch-type FARGATE \
  --network-configuration '{
    "awsvpcConfiguration": {
      "subnets": ["subnet-xxx"],
      "securityGroups": ["sg-xxx"],
      "assignPublicIp": "DISABLED"
    }
  }'
```

## Troubleshooting

### Migration Failed

Check CloudWatch Logs:
```bash
aws logs tail /ecs/omni-{customer} \
  --follow \
  --filter-pattern "migrator"
```

Check Lambda logs:
```bash
aws logs tail /aws/lambda/omni-{customer}-migrator --follow
```

### Task Won't Start

Verify:
- Task definition exists
- Subnets and security groups are correct
- IAM roles have proper permissions
- Database is accessible from ECS

### Migration Timeout

Default timeout: 10 minutes

If migrations take longer:
1. Update Lambda timeout (max 15 minutes)
2. Or split migrations into smaller batches
3. Or run migration task manually

## Re-running Migrations

The null_resource uses `timestamp()` trigger, so it runs on every apply.

To prevent re-running:
```bash
terraform apply -target=module.everything_except_migrations
```

Or comment out the null_resource temporarily.

## Dependencies

This module should be applied after:
- Database is created and accessible
- ECS cluster exists
- Task definition is registered
- Network configuration is complete

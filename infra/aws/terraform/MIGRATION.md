# CloudFormation to Terraform Migration Guide

This guide helps migrate from the existing CloudFormation deployment to the new Terraform-based deployment.

## Overview

### What's Changing

**Old Approach (CloudFormation)**:
- Customer runs bootstrap template
- Omni team assumes cross-account role
- Omni team deploys via CloudFormation
- Complex role management and permissions

**New Approach (Terraform)**:
- Customer creates dedicated AWS account
- Customer runs Terraform directly
- No cross-account roles needed
- Full visibility and control

### Benefits

1. **Simplified deployment** - No cross-account complexity
2. **Customer control** - Direct infrastructure management
3. **Better tooling** - Terraform ecosystem and plan/preview
4. **Modular design** - Reusable modules for multi-environment
5. **GitOps ready** - Easy CI/CD integration

## Migration Strategies

### Strategy 1: Fresh Deployment (Recommended)

Deploy new Terraform stack alongside existing CloudFormation.

**Pros**:
- Zero downtime
- Easy rollback
- Test before cutting over

**Cons**:
- Temporary duplicate infrastructure costs
- Data migration required

**Steps**:

1. **Deploy Terraform stack in new account**
   ```bash
   cd infra/aws/terraform
   cp terraform.tfvars.example terraform.tfvars
   # Edit terraform.tfvars
   ./scripts/deploy.sh
   ```

2. **Migrate data**
   ```bash
   # Export from CloudFormation database
   pg_dump -h old-db-endpoint -U omni -d omni > omni.sql

   # Import to Terraform database
   psql -h new-db-endpoint -U omni -d omni < omni.sql
   ```

3. **Test new deployment**
   - Verify all services are running
   - Test search functionality
   - Test integrations

4. **Update DNS**
   ```bash
   # Point domain to new ALB
   omni.company.com -> new-alb-dns-name
   ```

5. **Monitor and verify**
   - Watch CloudWatch Logs
   - Check ECS service health
   - Verify user access

6. **Destroy CloudFormation stack**
   ```bash
   aws cloudformation delete-stack --stack-name omni-customer-infrastructure
   ```

### Strategy 2: In-Place Migration (Advanced)

Import existing resources into Terraform state.

**Pros**:
- No duplicate infrastructure
- No data migration

**Cons**:
- Complex import process
- Risk of disruption
- Requires exact resource matching

**⚠️ Not recommended** - CloudFormation and Terraform manage resources differently. Fresh deployment is safer.

### Strategy 3: Parallel Operation

Run both stacks temporarily during transition.

**Steps**:

1. Deploy Terraform in separate environment (dev/staging)
2. Test thoroughly
3. Schedule maintenance window
4. Deploy Terraform to production
5. Migrate data
6. Switch traffic
7. Destroy CloudFormation

## Pre-Migration Checklist

- [ ] Create backup of CloudFormation database
- [ ] Export CloudFormation stack outputs
- [ ] Document current configuration
- [ ] List all custom modifications
- [ ] Review security groups and network setup
- [ ] Export CloudWatch dashboards and alarms
- [ ] Document DNS configuration
- [ ] List all secrets and API keys

## Data Migration

### Database Migration

```bash
# 1. Create snapshot of CloudFormation database
aws rds create-db-snapshot \
  --db-instance-identifier omni-customer-postgres \
  --db-snapshot-identifier omni-cfn-final-$(date +%Y%m%d)

# 2. Wait for snapshot completion
aws rds wait db-snapshot-completed \
  --db-snapshot-identifier omni-cfn-final-$(date +%Y%m%d)

# 3. Export database
pg_dump -h $OLD_DB_ENDPOINT \
  -U omni \
  -d omni \
  --no-owner \
  --no-acl \
  > omni_export.sql

# 4. Import to new database
psql -h $NEW_DB_ENDPOINT \
  -U omni \
  -d omni \
  < omni_export.sql

# 5. Verify data
psql -h $NEW_DB_ENDPOINT -U omni -d omni -c "SELECT COUNT(*) FROM documents;"
```

### Redis Data (Session Storage)

Redis data is typically transient and doesn't need migration.

Users will need to log in again after migration.

## Configuration Mapping

### CloudFormation → Terraform Variables

| CloudFormation Parameter | Terraform Variable |
|-------------------------|-------------------|
| `CustomerName` | `customer_name` |
| `Environment` | `environment` |
| `DBInstanceClass` | `db_instance_class` |
| `RedisNodeType` | `redis_node_type` |
| `ECSTaskCpu` | `ecs_task_cpu` |
| `ECSTaskMemory` | `ecs_task_memory` |
| `GitHubOrg` | `github_org` |
| `SSLCertificateArn` | `ssl_certificate_arn` |

### Secrets Migration

All secrets are recreated in Terraform:

```bash
# Get secrets from CloudFormation deployment
aws secretsmanager get-secret-value \
  --secret-id omni/customer/jina-api-key

# These are automatically created in Terraform:
# - Database password (new, auto-generated)
# - Encryption keys (new, auto-generated)
# - Session secret (new, auto-generated)

# Only JINA API key needs to be provided in terraform.tfvars
```

## DNS Update

### Before Migration

```
omni.company.com → old-cfn-alb-123.elb.amazonaws.com
```

### After Migration

```
omni.company.com → new-tf-alb-456.elb.amazonaws.com
```

### Update Steps

1. **Lower TTL** (24 hours before migration)
   ```bash
   # Set DNS TTL to 60 seconds
   ```

2. **Update record** (during migration)
   ```bash
   # Change CNAME/ALIAS to new ALB
   ```

3. **Monitor** (after migration)
   ```bash
   # Watch for DNS propagation
   dig omni.company.com

   # Monitor old ALB traffic (should decrease)
   aws cloudwatch get-metric-statistics ...
   ```

4. **Restore TTL** (after successful migration)
   ```bash
   # Increase TTL back to normal (3600)
   ```

## Rollback Plan

If issues occur after migration:

### Immediate Rollback

1. **Revert DNS**
   ```bash
   # Point back to CloudFormation ALB
   omni.company.com → old-cfn-alb-123.elb.amazonaws.com
   ```

2. **Verify old stack is healthy**
   ```bash
   aws ecs describe-services \
     --cluster omni-customer-cluster \
     --services omni-customer-web
   ```

3. **Communicate to users**
   - Send notification about temporary issue
   - Provide ETA for resolution

### Delayed Issues

If issues discovered after DNS propagation:

1. Keep Terraform stack running
2. Investigate and fix issues
3. Re-migrate when resolved
4. CloudFormation stack can serve as backup

## Post-Migration Tasks

- [ ] Update documentation with new endpoints
- [ ] Update monitoring dashboards
- [ ] Configure CloudWatch alarms
- [ ] Test disaster recovery procedures
- [ ] Update runbooks and procedures
- [ ] Train team on Terraform operations
- [ ] Set up CI/CD pipelines
- [ ] Review and optimize costs
- [ ] Schedule cleanup of old resources

## Cost Comparison

### During Migration (Both Stacks)

Expect ~2x normal costs while running both stacks.

### After Migration

Costs should be similar, but with better visibility:

```bash
# View costs by tag
aws ce get-cost-and-usage \
  --time-period Start=2025-01-01,End=2025-01-31 \
  --granularity MONTHLY \
  --metrics BlendedCost \
  --group-by Type=TAG,Key=ManagedBy
```

## Timeline Example

**Week 1-2: Preparation**
- Review current setup
- Plan migration strategy
- Create Terraform configuration
- Test in dev/staging

**Week 3: Migration Rehearsal**
- Deploy Terraform to dev
- Practice data migration
- Test rollback procedures
- Document process

**Week 4: Production Migration**
- Deploy Terraform to production
- Migrate data
- Switch DNS (low traffic window)
- Monitor closely

**Week 5: Stabilization**
- Monitor performance
- Fine-tune configuration
- Destroy CloudFormation stack
- Document lessons learned

## Troubleshooting

### Issue: Terraform Plan Shows Many Changes

**Cause**: State drift or import needed

**Solution**:
```bash
# Refresh state
terraform refresh

# Or start fresh (recommended)
rm -rf .terraform terraform.tfstate*
terraform init
terraform plan
```

### Issue: Services Won't Start in New Stack

**Cause**: Configuration mismatch

**Solution**:
```bash
# Compare configurations
aws cloudformation describe-stacks \
  --stack-name omni-customer-infrastructure

terraform show

# Check for differences in:
# - Environment variables
# - Security groups
# - Network configuration
```

### Issue: Database Connection Errors

**Cause**: Different credentials or endpoints

**Solution**:
```bash
# Verify database endpoint
terraform output database_endpoint

# Test connectivity
psql -h $(terraform output -raw database_endpoint) \
  -U omni \
  -d omni \
  -c "SELECT version();"
```

## Support During Migration

### Before Migration

- Test in non-production environment
- Document current setup thoroughly
- Create detailed migration plan

### During Migration

- Have rollback plan ready
- Monitor all services
- Keep communication channels open

### After Migration

- Monitor for 48-72 hours
- Keep CloudFormation stack for 1 week
- Document any issues and resolutions

## Frequently Asked Questions

**Q: Can I migrate without downtime?**

A: Yes, using fresh deployment strategy with DNS switchover.

**Q: Will user sessions be preserved?**

A: No, users will need to log in again after migration.

**Q: Do I need to reconfigure integrations?**

A: Yes, but it's the same configuration. Update DNS in Google OAuth, Resend, etc.

**Q: Can I roll back after migration?**

A: Yes, by reverting DNS to CloudFormation ALB (if kept running).

**Q: How long does migration take?**

A: Fresh deployment: ~30 minutes infrastructure + data migration time.
   DNS propagation: Up to 24 hours (usually much faster).

**Q: What if I customized CloudFormation templates?**

A: Review customizations and apply equivalent changes to Terraform modules.

## Next Steps

1. Review this guide thoroughly
2. Test Terraform deployment in dev/staging
3. Create detailed migration plan for your environment
4. Schedule migration window
5. Execute migration
6. Monitor and verify
7. Clean up old resources

For assistance, contact: support@getomni.co

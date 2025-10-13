# Load Balancer Module

This module creates an Application Load Balancer for Omni web service.

## Resources Created

- Application Load Balancer (internet-facing)
- Target Group (port 3000, health check /health)
- HTTP Listener (port 80)
  - Forwards to target group OR redirects to HTTPS
- HTTPS Listener (port 443, optional)
  - Requires SSL certificate ARN

## Usage

### HTTP-only (Development)
```hcl
module "loadbalancer" {
  source = "./modules/loadbalancer"

  customer_name     = "acme-corp"
  environment       = "development"
  vpc_id            = module.networking.vpc_id
  subnet_ids        = module.networking.public_subnet_ids
  security_group_id = module.networking.alb_security_group_id
}
```

### HTTPS with Certificate (Production)
```hcl
module "loadbalancer" {
  source = "./modules/loadbalancer"

  customer_name              = "acme-corp"
  environment                = "production"
  vpc_id                     = module.networking.vpc_id
  subnet_ids                 = module.networking.public_subnet_ids
  security_group_id          = module.networking.alb_security_group_id
  ssl_certificate_arn        = "arn:aws:acm:us-east-1:123456789012:certificate/..."
  enable_deletion_protection = true
}
```

## Inputs

| Name | Description | Type | Default | Required |
|------|-------------|------|---------|----------|
| customer_name | Customer name for resource naming | string | - | yes |
| environment | Environment (production, staging, development) | string | "production" | no |
| vpc_id | VPC ID | string | - | yes |
| subnet_ids | List of subnet IDs for ALB | list(string) | - | yes |
| security_group_id | Security group ID for ALB | string | - | yes |
| ssl_certificate_arn | ARN of ACM certificate for HTTPS | string | "" | no |
| enable_deletion_protection | Enable deletion protection for ALB | bool | false | no |

## Outputs

| Name | Description |
|------|-------------|
| dns_name | DNS name of the load balancer |
| arn | ARN of the load balancer |
| target_group_arn | ARN of the target group |
| http_listener_arn | ARN of HTTP listener |
| https_listener_arn | ARN of HTTPS listener (empty if no SSL) |
| zone_id | Zone ID of the load balancer |

## Load Balancer Configuration

### Listeners

**HTTP (Port 80)**
- Without SSL: Forwards traffic to target group
- With SSL: Redirects to HTTPS (301)

**HTTPS (Port 443)** - Optional
- Requires ACM certificate ARN
- TLS 1.3 policy (ELBSecurityPolicy-TLS13-1-2-2021-06)
- Forwards traffic to target group

### Target Group

- **Port**: 3000 (omni-web)
- **Protocol**: HTTP
- **Target Type**: IP (for Fargate)
- **Health Check**:
  - Path: `/health`
  - Interval: 30 seconds
  - Timeout: 5 seconds
  - Healthy threshold: 2
  - Unhealthy threshold: 5

### Deployment

- **Type**: Application Load Balancer
- **Scheme**: Internet-facing
- **Subnets**: Public subnets (multi-AZ)
- **Deletion Protection**: Disabled by default

## SSL/TLS Setup

### 1. Request Certificate in ACM

```bash
# Via AWS Console
# Navigate to: Certificate Manager > Request certificate
# Enter domain name (e.g., omni.company.com)
# Validate via DNS or email
```

### 2. Copy Certificate ARN

```bash
aws acm list-certificates --region us-east-1
```

### 3. Provide ARN to Terraform

```hcl
ssl_certificate_arn = "arn:aws:acm:us-east-1:123456789012:certificate/abcd-1234-..."
```

### 4. Update DNS

Point your domain to the ALB DNS name:

```
omni.company.com. CNAME omni-customer-alb-1234567890.us-east-1.elb.amazonaws.com.
```

Or use Route53 Alias record for better performance.

## Accessing the Application

### Without SSL
```
http://<alb-dns-name>
```

### With SSL
```
https://omni.company.com
```

HTTP requests will automatically redirect to HTTPS.

## Monitoring

The ALB provides these CloudWatch metrics:
- Request count
- Target response time
- HTTP 4xx/5xx errors
- Active connection count
- Healthy/unhealthy target count

View metrics in CloudWatch Console or set up alarms.

## Cost

ALB pricing includes:
- Hourly charge (~$16-22/month)
- Per GB processed charge
- LCU (Load Balancer Capacity Unit) charges

See [AWS ALB Pricing](https://aws.amazon.com/elasticloadbalancing/pricing/) for details.

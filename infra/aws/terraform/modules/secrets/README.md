# Secrets Module

This module creates and manages secrets in AWS Secrets Manager for Omni infrastructure.

## Resources Created

- Database password secret (auto-generated, 48 characters, alphanumeric)
- Embedding API key secret (provided via variable)
- Encryption key secret (auto-generated, 64 characters)
- Encryption salt secret (auto-generated, 32 characters)

## Usage

```hcl
module "secrets" {
  source = "./modules/secrets"

  customer_name     = "acme-corp"
  environment       = "production"
  database_username = "omni"
  embedding_api_key      = var.embedding_api_key
}
```

## Inputs

| Name | Description | Type | Default | Required |
|------|-------------|------|---------|----------|
| customer_name | Customer name for resource naming | string | - | yes |
| environment | Environment (production, staging, development) | string | "production" | no |
| database_username | PostgreSQL master username | string | "omni" | no |
| embedding_api_key | Embedding API key for embedding generation | string | - | yes |

## Outputs

| Name | Description |
|------|-------------|
| database_password_arn | ARN of database password secret |
| database_password | Database password value (sensitive) |
| embedding_api_key_arn | ARN of Embedding API key secret |
| encryption_key_arn | ARN of encryption key secret |
| encryption_salt_arn | ARN of encryption salt secret |

## Secret Format

### Database Password
```json
{
  "username": "omni",
  "password": "<generated-password>"
}
```

### Embedding API Key
```
<api-key-string>
```

### Encryption Key
```json
{
  "key": "<generated-key>"
}
```

### Encryption Salt
```json
{
  "salt": "<generated-salt>"
}
```

## Security Considerations

- All secrets are encrypted at rest using AWS KMS
- Passwords are auto-generated using Terraform's random_password resource
- Sensitive outputs are marked as sensitive to prevent accidental exposure
- Embedding API key must be provided as a variable and should be stored securely

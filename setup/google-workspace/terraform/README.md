# Omni Google Workspace Integration - Terraform Setup

This Terraform configuration automates the Google Cloud setup for Omni's Google Workspace connector. It creates a GCP project, service account, and enables the required APIs.

For the full integration guide, see the [Google Workspace connector documentation](https://docs.getomni.co/connectors/google-workspace).

## Prerequisites

1. **Google Cloud CLI** installed and authenticated:
   ```bash
   gcloud auth login
   gcloud auth application-default login
   ```

2. **Terraform** (>= 1.0) installed

3. A **billing account** set up in Google Cloud

## Quick Start

1. Copy the example configuration:
   ```bash
   cp terraform.tfvars.example terraform.tfvars
   ```

2. Edit `terraform.tfvars` with your values:
   ```hcl
   workspace_domain     = "your-company.com"
   admin_email          = "admin@your-company.com"
   billing_account_name = "My Billing Account"
   ```

3. Run Terraform:
   ```bash
   terraform init
   terraform plan
   terraform apply
   ```

4. Follow the manual steps in the output (or in the generated `SETUP_INSTRUCTIONS.md`).

## What Gets Created

**Google Cloud resources:**
- A new GCP project with billing enabled
- Required APIs enabled (Admin SDK, Drive, Gmail, Docs, Sheets, Slides)
- A service account for domain-wide delegation
- A service account key (saved locally)

**Local files:**
- `omni-service-account-key.json` — service account credentials
- `SETUP_INSTRUCTIONS.md` — setup guide with your specific values

## Configuration

### Required Variables

| Variable | Description |
|----------|-------------|
| `workspace_domain` | Your Google Workspace domain (e.g., `company.com`) |
| `admin_email` | A Google Workspace admin email address |
| `billing_account_name` | Name of the GCP billing account to use |

### Optional Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `project_name` | `omni-workspace-integration` | GCP project name |
| `project_id` | (auto-generated) | Specific GCP project ID |
| `include_gmail_scope` | `true` | Include Gmail read access in OAuth scopes |
| `output_key_file` | `true` | Save the service account key to a local file |
| `manage_org_policy` | `false` | Create org-level tags and policy to allow SA key creation (requires Organization Admin permissions — see below) |

### Organization Policy (optional)

If your GCP organization blocks service account key creation (`iam.disableServiceAccountKeyCreation`), set `manage_org_policy = true`. This creates org-level tags and a conditional policy to allow key creation for this project only.

This requires **Organization Admin** permissions and may conflict with existing org policies. Most users won't need this.

## Manual Steps

Domain-wide delegation cannot be configured via Terraform. After `terraform apply`:

1. Go to the [Google Workspace Admin Console](https://admin.google.com/ac/owl/domainwidedelegation)
2. Add the OAuth Client ID shown in the Terraform output
3. Add the OAuth scopes shown in the Terraform output
4. Configure the connector in Omni with the generated service account key

See the [full setup guide](https://docs.getomni.co/connectors/google-workspace) for details.

## Managing Resources

**Rotate service account key:**
```bash
terraform taint google_service_account_key.omni_sa_key
terraform apply
```

**Tear down:**
```bash
terraform destroy
```

## Troubleshooting

**"Billing account not found"**
```bash
gcloud billing accounts list
```
Use the exact display name in `billing_account_name`.

**"Organization not found"**
```bash
gcloud organizations list
```
Verify you're authenticated with an account that has access to the organization.

**"Insufficient permissions"**
- The base setup requires Project Creator permissions
- `manage_org_policy = true` additionally requires Organization Admin

**"API not enabled"**
- APIs are enabled automatically but may take a few minutes to propagate

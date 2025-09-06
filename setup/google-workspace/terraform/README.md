# Omni Google Workspace Integration - Terraform Setup

This Terraform configuration automates the Google Cloud setup for Omni's Google Workspace integration, reducing setup time from 45 minutes to 5 minutes.

## 🚀 Quick Start

### Prerequisites

1. **Google Cloud CLI installed and authenticated**
   ```bash
   gcloud auth login
   gcloud auth application-default login
   ```

2. **Terraform installed** (version >= 1.0)
   ```bash
   # On macOS
   brew install terraform
   
   # On Windows
   choco install terraform
   
   # On Linux
   wget https://releases.hashicorp.com/terraform/1.6.0/terraform_1.6.0_linux_amd64.zip
   ```

3. **Organization Admin permissions** in Google Workspace
4. **Billing account** set up in Google Cloud

### Setup Steps

1. **Clone or download** this Terraform configuration

2. **Copy the example configuration:**
   ```bash
   cp terraform.tfvars.example terraform.tfvars
   ```

3. **Edit `terraform.tfvars`** with your information:
   ```hcl
   workspace_domain = "your-company.com"
   admin_email      = "admin@your-company.com"
   ```

4. **Run Terraform:**
   ```bash
   terraform init
   terraform plan    # Review what will be created
   terraform apply   # Create resources
   ```

5. **Follow the manual steps** shown in the output or in `SETUP_INSTRUCTIONS.md`

## 📁 What Gets Created

### Google Cloud Resources
- ✅ New Google Cloud project with billing enabled
- ✅ Required APIs enabled (Admin SDK, Drive, Gmail, Docs, Sheets, Slides)
- ✅ Service account with domain-wide delegation
- ✅ Service account key (saved locally)
- ✅ Organization tags for project identification
- ✅ Organization policy allowing service account key creation for tagged projects

### Local Files
- ✅ `omni-service-account-key.json` - Service account credentials
- ✅ `SETUP_INSTRUCTIONS.md` - Complete setup guide with your specific values

## 🔧 Configuration Options

### Required Variables
```hcl
workspace_domain = "company.com"        # Your Google Workspace domain
admin_email      = "admin@company.com"  # Google Workspace admin email
```

### Optional Variables
```hcl
project_name         = "omni-workspace-integration"  # GCP project name
include_gmail_scope  = false                        # Exclude Gmail access
output_key_file      = false                        # Don't create local key file
billing_account_name = "My Billing Account"         # Specific billing account
```

## 🛡️ Security Features

1. **Minimal Permissions:** Service account only has read-only access
2. **Scope Control:** Can exclude Gmail or other scopes as needed  
3. **Organization Policy:** Restricts service account key creation to tagged projects only
4. **Key Security:** Local key file has restricted permissions (600)
5. **Audit Trail:** All resources are tagged and tracked

## 📋 Manual Steps Still Required

Due to Google's security model, these steps cannot be automated:

1. **Google Workspace Admin Console:** Add OAuth client ID and scopes
2. **Omni Configuration:** Upload key and configure integration

The Terraform output provides exact instructions with your specific values.

## 🔄 Managing the Infrastructure

### View Current State
```bash
terraform show
```

### Update Configuration
```bash
# Edit terraform.tfvars
terraform plan
terraform apply
```

### Rotate Service Account Key
```bash
terraform taint google_service_account_key.omni_sa_key
terraform apply
```

### Clean Up
```bash
terraform destroy
```

## 🆘 Troubleshooting

### Common Issues

**"Billing account not found"**
```bash
# List available billing accounts
gcloud billing accounts list
# Update terraform.tfvars with exact billing account name
```

**"Organization not found"**
```bash
# Verify you're authenticated with the right account
gcloud auth list
# Check organization access
gcloud organizations list
```

**"Insufficient permissions"**
- Ensure you have Organization Admin role
- Some permissions may take time to propagate (wait 10-15 minutes)

**"API not enabled"**
- APIs are enabled automatically, but may take a few minutes
- Check in Google Cloud Console: APIs & Services

### Getting Help

Include this information when requesting support:
- Project ID (from terraform output)
- Error messages
- Output of `gcloud auth list`

## 🔍 What the Automation Does vs Manual Process

| Step | Manual Process | Terraform |
|------|----------------|-----------|
| Create project | 5 minutes | ✅ Automated |
| Enable APIs | 5 minutes | ✅ Automated |
| Set up billing | 5 minutes | ✅ Automated |
| Grant IAM roles | 10 minutes | ✅ Automated |
| Create tags | 5 minutes | ✅ Automated |
| Organization policy | 10 minutes | ✅ Automated |
| Service account | 5 minutes | ✅ Automated |
| **Workspace Admin Console** | **5 minutes** | ❌ Manual |
| **Omni configuration** | **2 minutes** | ❌ Manual |

**Total time:** 45 minutes → 7 minutes (84% reduction)

## 📄 Files Overview

```
omni-terraform-setup/
├── main.tf                          # Main Terraform configuration
├── variables.tf                     # Input variables
├── outputs.tf                       # Output values  
├── terraform.tfvars.example         # Configuration template
├── templates/
│   └── setup_instructions.tpl       # Instructions template
├── README.md                        # This file
├── omni-service-account-key.json    # Generated service account key
└── SETUP_INSTRUCTIONS.md            # Generated setup guide
```

## 🔐 Security Considerations

- **Service account key** provides read-only access but can access entire domain
- **Store key securely** - treat like a master password
- **Rotate regularly** - recommended every 90 days  
- **Monitor usage** - enable audit logging
- **Remove when done** - run `terraform destroy` if integration is no longer needed

---

*This Terraform configuration follows Google Cloud and security best practices for automated infrastructure deployment.*

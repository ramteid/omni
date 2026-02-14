# Google Workspace Connector - Setup Instructions

Generated on ${timestamp()} by Terraform.

For the full integration guide, see https://docs.getomni.co/connectors/google-workspace

## Provisioned Resources

| Resource | Value |
|----------|-------|
| GCP Project | `${project_id}` |
| Service Account | `${service_account}` |
| OAuth Client ID | `${client_id}` |
%{ if key_file_created ~}
| Service Account Key | `omni-service-account-key.json` |
%{ endif ~}

## Remaining Steps

The following steps cannot be automated and must be completed manually.

### 1. Configure Domain-Wide Delegation

1. Open the [Google Workspace Admin Console](https://admin.google.com/ac/owl/domainwidedelegation)
2. Click **Add new** client ID
3. Enter the Client ID: `${client_id}`
4. Enter the following OAuth scopes:
   ```
   ${oauth_scopes}
   ```
5. Click **Authorize**

### 2. Configure Omni

1. Go to the Omni integrations page
2. Click **Connect Google Workspace**
3. Upload the service account key%{ if key_file_created } (`omni-service-account-key.json`)%{ endif }
4. Enter the admin email: `${admin_email}`
5. Enter the domain: `${workspace_domain}`

## Security Notes

This service account has **read-only** access to:
- User directory information (Admin SDK)
- Google Drive files and metadata
- Google Docs, Sheets, and Slides content
- Gmail messages (if the Gmail scope was included)

The service account key should be treated as a sensitive credential. Store it securely and rotate it periodically.

# Omni Google Workspace Integration - Setup Complete! 🎉

## Automated Setup Results

✅ **Google Cloud Project Created:** `${project_id}`
✅ **Service Account Created:** `${service_account}`
✅ **APIs Enabled:** Admin SDK, Drive, Gmail, Docs, Sheets, Slides
✅ **Organization Policy Configured:** Service account key creation allowed for tagged projects
✅ **Tags Applied:** Project tagged for Omni integration
%{ if key_file_created }✅ **Service Account Key:** Saved to `omni-service-account-key.json`%{ endif }

## 📋 Manual Steps Required

### Step 1: Configure Google Workspace Admin Console

1. **Go to:** [Google Workspace Admin Console](https://admin.google.com/ac/owl/domainwidedelegation)
2. **Click:** "Add new" client ID
3. **Enter Client ID:** `${client_id}`
4. **Enter OAuth Scopes:**
   ```
   ${oauth_scopes}
   ```
   > **Note:** Remove `gmail.readonly` if you don't want Omni to access Gmail inboxes
5. **Click:** "Authorize"

### Step 2: Configure Omni Integration

1. **Go to:** Omni integrations page
2. **Click:** "Connect Google Workspace" 
3. **Upload/Paste:** The service account key%{ if key_file_created } from `omni-service-account-key.json`%{ endif }
4. **Enter Admin Email:** `${admin_email}`
5. **Enter Domain:** `${workspace_domain}`

## 🔒 Security Information

This integration provides **read-only** access to:
- ✅ User directory information
- ✅ Group membership  
- ✅ Google Drive files
- ✅ Gmail messages (if enabled)
- ✅ Google Docs, Sheets, Slides content

The service account **cannot**:
- ❌ Modify or delete any data
- ❌ Send emails
- ❌ Create or edit documents
- ❌ Change user permissions

## 🛡️ Security Best Practices

1. **Store the key securely** - treat it like a password
2. **Rotate the key every 90 days**
3. **Monitor usage** through Google Cloud Console audit logs
4. **Remove access** if no longer needed

## 🔧 Troubleshooting

If you encounter issues:

1. **Wait 10-15 minutes** for all policies to propagate
2. **Verify billing** is enabled on the project
3. **Check permissions** - you need Organization Admin rights
4. **Contact support** with the project ID: `${project_id}`

## 📞 Support

- **Project ID:** `${project_id}`
- **Service Account:** `${service_account}`
- **Setup Date:** ${timestamp()}

---
*This setup was automated using Terraform. All resources are tagged and tracked.*

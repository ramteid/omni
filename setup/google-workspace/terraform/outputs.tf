output "project_id" {
  description = "The ID of the created Google Cloud project"
  value       = google_project.omni_project.project_id
}

output "project_number" {
  description = "The number of the created Google Cloud project"
  value       = google_project.omni_project.number
}

output "service_account_email" {
  description = "Email address of the created service account"
  value       = google_service_account.omni_sa.email
}

output "service_account_client_id" {
  description = "OAuth2 Client ID for domain-wide delegation"
  value       = google_service_account.omni_sa.unique_id
}

output "service_account_key_id" {
  description = "The ID of the service account key"
  value       = google_service_account_key.omni_sa_key.id
  sensitive   = true
}

output "service_account_private_key" {
  description = "The private key of the service account (base64 encoded)"
  value       = google_service_account_key.omni_sa_key.private_key
  sensitive   = true
}

output "oauth_scopes" {
  description = "OAuth scopes to configure in Google Workspace Admin Console"
  value       = local.oauth_scopes
}

output "workspace_admin_console_url" {
  description = "URL to Google Workspace Admin Console for domain-wide delegation setup"
  value       = "https://admin.google.com/ac/owl/domainwidedelegation"
}

output "tag_key_id" {
  description = "The ID of the created tag key"
  value       = google_tags_tag_key.omni_integration.id
}

output "tag_value_id" {
  description = "The ID of the created tag value"
  value       = google_tags_tag_value.allowed.id
}

output "setup_complete" {
  description = "Confirmation that automated setup is complete"
  value = <<-EOT
    ✅ Automated setup complete!
    
    📋 Manual steps remaining:
    1. Go to Google Workspace Admin Console: ${output.workspace_admin_console_url.value}
    2. Add OAuth Client ID: ${google_service_account.omni_sa.unique_id}
    3. Add OAuth Scopes: ${join(", ", local.oauth_scopes)}
    4. Configure Omni with the generated service account key
    
    📁 Files created:
    - omni-service-account-key.json (if output_key_file = true)
    - SETUP_INSTRUCTIONS.md
  EOT
}

output "next_steps" {
  description = "Next steps for completing the setup"
  value = {
    workspace_admin_console = {
      url                = "https://admin.google.com/ac/owl/domainwidedelegation"
      oauth_client_id    = google_service_account.omni_sa.unique_id
      oauth_scopes       = local.oauth_scopes
    }
    omni_configuration = {
      admin_email        = var.admin_email
      workspace_domain   = var.workspace_domain
      service_account_key_file = var.output_key_file ? "omni-service-account-key.json" : "Use the private_key output"
    }
  }
}

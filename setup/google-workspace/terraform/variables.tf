variable "project_name" {
  description = "Name for the Google Cloud project"
  type        = string
  default     = "omni-workspace-integration"
}

variable "project_id" {
  description = "Specific project ID to use (leave empty to auto-generate)"
  type        = string
  default     = ""
}

variable "workspace_domain" {
  description = "Google Workspace domain (e.g., company.com)"
  type        = string
  validation {
    condition     = can(regex("^[a-zA-Z0-9][a-zA-Z0-9-]{0,61}[a-zA-Z0-9]\\.[a-zA-Z]{2,}$", var.workspace_domain))
    error_message = "Please provide a valid domain name (e.g., company.com)."
  }
}

variable "admin_email" {
  description = "Google Workspace admin email address"
  type        = string
  validation {
    condition     = can(regex("^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$", var.admin_email))
    error_message = "Please provide a valid email address."
  }
}

variable "billing_account_name" {
  description = "Name of the billing account to use"
  type        = string
  default     = ""
}

variable "region" {
  description = "Default region for resources"
  type        = string
  default     = "us-central1"
}

variable "service_account_name" {
  description = "Name for the service account"
  type        = string
  default     = "omni-service-account"
  validation {
    condition     = can(regex("^[a-z]([a-z0-9-]*[a-z0-9])?$", var.service_account_name))
    error_message = "Service account name must start with a lowercase letter, followed by lowercase letters, numbers, or hyphens."
  }
}

variable "tag_key_name" {
  description = "Name for the organization tag key"
  type        = string
  default     = "omni-integration"
}

variable "tag_value_name" {
  description = "Name for the tag value"
  type        = string
  default     = "allowed"
}

variable "include_gmail_scope" {
  description = "Whether to include Gmail read access in OAuth scopes"
  type        = bool
  default     = true
}

variable "output_key_file" {
  description = "Whether to output the service account key to a local file"
  type        = bool
  default     = true
}

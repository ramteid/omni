resource "aws_service_discovery_service" "web" {
  name = "web"

  dns_config {
    namespace_id = var.service_discovery_namespace_id

    dns_records {
      ttl  = 300
      type = "A"
    }
  }

  health_check_custom_config {
    failure_threshold = 1
  }
}

resource "aws_service_discovery_service" "searcher" {
  name = "searcher"

  dns_config {
    namespace_id = var.service_discovery_namespace_id

    dns_records {
      ttl  = 300
      type = "A"
    }
  }

  health_check_custom_config {
    failure_threshold = 1
  }
}

resource "aws_service_discovery_service" "indexer" {
  name = "indexer"

  dns_config {
    namespace_id = var.service_discovery_namespace_id

    dns_records {
      ttl  = 300
      type = "A"
    }
  }

  health_check_custom_config {
    failure_threshold = 1
  }
}

resource "aws_service_discovery_service" "ai" {
  name = "ai"

  dns_config {
    namespace_id = var.service_discovery_namespace_id

    dns_records {
      ttl  = 300
      type = "A"
    }
  }

  health_check_custom_config {
    failure_threshold = 1
  }
}

resource "aws_service_discovery_service" "google_connector" {
  name = "google-connector"

  dns_config {
    namespace_id = var.service_discovery_namespace_id

    dns_records {
      ttl  = 300
      type = "A"
    }
  }

  health_check_custom_config {
    failure_threshold = 1
  }
}

resource "aws_service_discovery_service" "atlassian_connector" {
  name = "atlassian-connector"

  dns_config {
    namespace_id = var.service_discovery_namespace_id

    dns_records {
      ttl  = 300
      type = "A"
    }
  }

  health_check_custom_config {
    failure_threshold = 1
  }
}

resource "aws_service_discovery_service" "web_connector" {
  name = "web-connector"

  dns_config {
    namespace_id = var.service_discovery_namespace_id

    dns_records {
      ttl  = 300
      type = "A"
    }
  }

  health_check_custom_config {
    failure_threshold = 1
  }
}

resource "aws_service_discovery_service" "connector_manager" {
  name = "connector-manager"

  dns_config {
    namespace_id = var.service_discovery_namespace_id

    dns_records {
      ttl  = 300
      type = "A"
    }
  }

  health_check_custom_config {
    failure_threshold = 1
  }
}

resource "aws_service_discovery_service" "slack_connector" {
  name = "slack-connector"

  dns_config {
    namespace_id = var.service_discovery_namespace_id

    dns_records {
      ttl  = 300
      type = "A"
    }
  }

  health_check_custom_config {
    failure_threshold = 1
  }
}

resource "aws_service_discovery_service" "github_connector" {
  name = "github-connector"

  dns_config {
    namespace_id = var.service_discovery_namespace_id

    dns_records {
      ttl  = 300
      type = "A"
    }
  }

  health_check_custom_config {
    failure_threshold = 1
  }
}

resource "aws_service_discovery_service" "hubspot_connector" {
  name = "hubspot-connector"

  dns_config {
    namespace_id = var.service_discovery_namespace_id

    dns_records {
      ttl  = 300
      type = "A"
    }
  }

  health_check_custom_config {
    failure_threshold = 1
  }
}

resource "aws_service_discovery_service" "microsoft_connector" {
  name = "microsoft-connector"

  dns_config {
    namespace_id = var.service_discovery_namespace_id

    dns_records {
      ttl  = 300
      type = "A"
    }
  }

  health_check_custom_config {
    failure_threshold = 1
  }
}

resource "aws_service_discovery_service" "notion_connector" {
  name = "notion-connector"

  dns_config {
    namespace_id = var.service_discovery_namespace_id

    dns_records {
      ttl  = 300
      type = "A"
    }
  }

  health_check_custom_config {
    failure_threshold = 1
  }
}

resource "aws_service_discovery_service" "fireflies_connector" {
  name = "fireflies-connector"

  dns_config {
    namespace_id = var.service_discovery_namespace_id

    dns_records {
      ttl  = 300
      type = "A"
    }
  }

  health_check_custom_config {
    failure_threshold = 1
  }
}

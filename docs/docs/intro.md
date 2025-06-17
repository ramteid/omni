---
sidebar_position: 1
---

# Clio Deployment Guide

**Enterprise AI Search Platform for IT Teams**

Clio is a self-hosted enterprise search platform that provides unified search across your organization's data sources with AI-powered answers. This documentation is designed to help IT teams deploy, configure, and maintain Clio in production environments.

## What is Clio?

Clio consolidates search across multiple enterprise data sources:
- **Google Workspace** (Drive, Docs, Gmail)
- **Slack** (messages, files, channels)  
- **Confluence** (pages, attachments, spaces)
- **GitHub** (repositories, issues, pull requests)
- **Local file systems**

Unlike cloud-based solutions like Glean, Clio runs entirely on your infrastructure, ensuring complete data privacy and control.

## Deployment Overview

Clio uses a **Docker Compose** architecture with these components:

| Service | Purpose | Technology |
|---------|---------|------------|
| **clio-web** | Frontend & API Gateway | SvelteKit/Node.js |
| **clio-searcher** | Search engine | Rust |
| **clio-indexer** | Document processing | Rust |  
| **clio-ai** | AI/ML services | Python/FastAPI |
| **PostgreSQL** | Primary database | PostgreSQL 17+ |
| **Redis** | Cache & message queue | Redis 7+ |
| **Caddy** | Load balancer & SSL | Caddy |
| **vLLM** | Local LLM inference | Python |

## Quick Deployment

Get Clio running in **under 10 minutes**:

```bash
# Clone the repository
git clone https://github.com/cliohq/clio.git
cd clio

# Start all services
docker compose up -d

# Access at https://localhost (or your domain)
```

→ **[Start here: Docker Deployment Guide](./getting-started/docker-deployment)**

## Production Checklist

Before deploying Clio in production:

- [ ] **System Requirements**: Verify hardware and software requirements
- [ ] **SSL Certificates**: Configure HTTPS with your certificates  
- [ ] **Database Setup**: Configure PostgreSQL with appropriate resources
- [ ] **Backup Strategy**: Set up automated database backups
- [ ] **Monitoring**: Configure health checks and alerting
- [ ] **Security**: Review authentication and network security
- [ ] **Data Sources**: Plan OAuth configurations for integrations

→ **[Production Setup Guide](./deployment/production-setup)**

## Architecture for IT Teams

Understanding Clio's architecture helps with:
- **Resource planning** and sizing
- **Network configuration** and security
- **Backup and disaster recovery** planning
- **Monitoring and troubleshooting**

→ **[Architecture Overview](./architecture/overview)**

## Support & Maintenance

- **Monitoring**: Health checks, metrics, and alerting
- **Updates**: Rolling updates and version management  
- **Backup/Restore**: Database and configuration backup strategies
- **Troubleshooting**: Common issues and debugging guides

→ **[Operations Guide](./operations/monitoring)**

## Getting Help

- **GitHub Issues**: [Report deployment issues](https://github.com/cliohq/clio/issues)
- **Discussions**: [Ask questions](https://github.com/cliohq/clio/discussions)
- **Documentation**: Search this documentation for specific topics

---

**Ready to deploy?** Start with the [System Requirements](./getting-started/system-requirements) to plan your deployment.

# Docker Deployment

Get Clio running quickly using Docker Compose. This guide covers development/testing deployments. For production, see the [Production Setup Guide](../deployment/production-setup).

## Prerequisites

Ensure you have the required software installed:

```bash
# Check Docker version (requires 24.0+)
docker --version

# Check Docker Compose version (requires 2.20+)
docker compose version

# Check Git
git --version
```

## Quick Start

### 1. Clone the Repository

```bash
git clone https://github.com/cliohq/clio.git
cd clio
```

### 2. Start All Services

```bash
# Start all services in background
docker compose up -d

# Check status
docker compose ps
```

### 3. Access Clio

Open your browser and navigate to:
- **HTTP**: http://localhost
- **HTTPS**: https://localhost (with self-signed certificate)

The first startup takes 5-10 minutes as Docker images are downloaded and built.

## Docker Compose Files

Clio uses multiple compose files for different environments:

| File | Purpose |
|------|---------|
| `docker-compose.yml` | Base configuration |
| `docker-compose.dev.yml` | Development overrides |
| `docker-compose.prod.yml` | Production overrides |

### Development Mode

```bash
# Run with development settings
docker compose -f docker-compose.yml -f docker-compose.dev.yml up -d
```

Development mode includes:
- Hot reload for web interface
- Debug logging enabled
- Development database with sample data

### Production Mode

```bash
# Run with production settings  
docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d
```

## Service Overview

When you run `docker compose up -d`, these services start:

| Service | Container Name | Purpose |
|---------|----------------|---------|
| **clio-web** | clio-web | Frontend and API gateway |
| **clio-searcher** | clio-searcher | Search processing |
| **clio-indexer** | clio-indexer | Document indexing |
| **clio-ai** | clio-ai | AI/ML processing |
| **postgres** | clio-postgres | Primary database |
| **redis** | clio-redis | Cache and message queue |
| **caddy** | clio-caddy | Load balancer and SSL |
| **vllm** | clio-vllm | Local LLM inference |

## Initial Setup

### 1. Create Admin User

```bash
# Create the first admin user
docker compose exec clio-web npm run create-admin-user
```

### 2. Configure Data Sources

Log in as admin and configure your first data source:
1. Go to **Settings** → **Integrations**
2. Click **Connect Google Workspace**
3. Follow the OAuth flow

## Common Commands

### View Logs

```bash
# All services
docker compose logs

# Specific service
docker compose logs clio-searcher

# Follow logs in real-time
docker compose logs -f clio-web
```

### Service Management

```bash
# Stop all services
docker compose down

# Restart a specific service
docker compose restart clio-searcher

# Rebuild and restart
docker compose up -d --build clio-web
```

### Database Access

```bash
# Connect to PostgreSQL
docker compose exec postgres psql -U clio -d clio

# Run a backup
docker compose exec postgres pg_dump -U clio clio > backup.sql
```

### Monitoring

```bash
# Check resource usage
docker compose top

# View container stats
docker stats
```

## Configuration

### Environment Variables

Create a `.env` file in the project root:

```bash
# Database
DATABASE_URL=postgresql://clio:clio_password@postgres:5432/clio

# Redis
REDIS_URL=redis://redis:6379

# Security
JWT_SECRET=your-secure-random-string-here
ENCRYPTION_KEY=your-32-char-encryption-key-here

# External URLs (for OAuth callbacks)
BASE_URL=https://your-domain.com

# AI Service
VLLM_URL=http://vllm:8000
```

### Custom Domains

To use a custom domain, update the `Caddyfile`:

```caddyfile
your-domain.com {
    reverse_proxy clio-web:3000
}
```

## Troubleshooting

### Container Won't Start

```bash
# Check container status
docker compose ps

# View detailed logs
docker compose logs <service-name>

# Check resource usage
docker system df
```

### Performance Issues

```bash
# Monitor resource usage
docker stats

# Check disk space
df -h

# View database performance
docker compose exec postgres pg_stat_activity
```

### Database Issues

```bash
# Reset database (⚠️ destroys all data)
docker compose down
docker volume rm clio_postgres_data
docker compose up -d

# Run database migrations manually
docker compose exec clio-indexer cargo run --bin migrate
```

### Network Issues

```bash
# Check network connectivity
docker compose exec clio-web wget -qO- http://clio-searcher:8080/health

# Inspect Docker networks
docker network ls
docker network inspect clio_default
```

## Health Checks

All services include health checks. Check status:

```bash
# Overall health
docker compose ps

# Individual service health
curl -f http://localhost/api/health
```

## Data Persistence

Important data is stored in Docker volumes:

- **postgres_data**: Database files
- **redis_data**: Cache data  
- **caddy_data**: SSL certificates

To backup data:

```bash
# Create volume backup
docker run --rm -v clio_postgres_data:/data -v $(pwd):/backup alpine tar czf /backup/postgres_backup.tar.gz /data
```

## Next Steps

Once Clio is running:

1. **[Production Setup](../deployment/production-setup)** - Prepare for production
2. **[Monitoring](../operations/monitoring)** - Set up monitoring and alerts
3. **[Architecture Overview](../architecture/overview)** - Learn about the system design
#!/bin/sh
set -e

PROJECT_NAME="${COMPOSE_PROJECT_NAME:-omni}"

docker compose -p "$PROJECT_NAME" -f docker/docker-compose.yml --env-file .env build
docker compose -p "$PROJECT_NAME" -f docker/docker-compose.yml --env-file .env up -d --no-build
docker compose -p "$PROJECT_NAME" -f docker/docker-compose.yml --env-file .env logs -f

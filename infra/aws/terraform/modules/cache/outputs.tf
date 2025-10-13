output "endpoint" {
  description = "Redis cluster endpoint address"
  value       = aws_elasticache_cluster.redis.cache_nodes[0].address
}

output "port" {
  description = "Redis cluster port"
  value       = aws_elasticache_cluster.redis.port
}

output "cluster_id" {
  description = "Redis cluster ID"
  value       = aws_elasticache_cluster.redis.id
}

output "arn" {
  description = "Redis cluster ARN"
  value       = aws_elasticache_cluster.redis.arn
}

output "vpc_id" {
  description = "ID of the VPC"
  value       = aws_vpc.main.id
}

output "public_subnet_ids" {
  description = "IDs of public subnets"
  value       = aws_subnet.public[*].id
}

output "private_subnet_ids" {
  description = "IDs of private subnets"
  value       = aws_subnet.private[*].id
}

output "alb_security_group_id" {
  description = "ID of ALB security group"
  value       = aws_security_group.alb.id
}

output "ecs_security_group_id" {
  description = "ID of ECS security group"
  value       = aws_security_group.ecs.id
}

output "database_security_group_id" {
  description = "ID of database security group"
  value       = aws_security_group.database.id
}

output "redis_security_group_id" {
  description = "ID of Redis security group"
  value       = aws_security_group.redis.id
}

output "nat_gateway_id" {
  description = "ID of NAT Gateway"
  value       = aws_nat_gateway.main.id
}

output "dns_name" {
  description = "DNS name of the load balancer"
  value       = aws_lb.main.dns_name
}

output "arn" {
  description = "ARN of the load balancer"
  value       = aws_lb.main.arn
}

output "target_group_arn" {
  description = "ARN of the target group"
  value       = aws_lb_target_group.main.arn
}

output "http_listener_arn" {
  description = "ARN of HTTP listener"
  value       = aws_lb_listener.http.arn
}

output "https_listener_arn" {
  description = "ARN of HTTPS listener (empty if no SSL)"
  value       = length(aws_lb_listener.https) > 0 ? aws_lb_listener.https[0].arn : ""
}

output "zone_id" {
  description = "Zone ID of the load balancer"
  value       = aws_lb.main.zone_id
}

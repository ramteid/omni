locals {
  common_tags = {
    Application = "Omni"
    Customer    = var.customer_name
    Environment = var.environment
    ManagedBy   = "Terraform"
  }

  has_ssl_certificate = var.ssl_certificate_arn != ""
}

resource "aws_lb" "main" {
  name               = "omni-${var.customer_name}-alb"
  internal           = false
  load_balancer_type = "application"
  security_groups    = [var.security_group_id]
  subnets            = var.subnet_ids

  enable_deletion_protection = var.enable_deletion_protection

  tags = merge(local.common_tags, {
    Name = "omni-${var.customer_name}-alb"
  })
}

resource "aws_lb_target_group" "main" {
  name        = "omni-${var.customer_name}-tg"
  port        = 3000
  protocol    = "HTTP"
  vpc_id      = var.vpc_id
  target_type = "ip"

  health_check {
    enabled             = true
    healthy_threshold   = 2
    unhealthy_threshold = 5
    timeout             = 5
    interval            = 30
    path                = "/health"
    protocol            = "HTTP"
    matcher             = "200"
  }

  tags = merge(local.common_tags, {
    Name = "omni-${var.customer_name}-tg"
  })
}

resource "aws_lb_listener" "http" {
  load_balancer_arn = aws_lb.main.arn
  port              = 80
  protocol          = "HTTP"

  default_action {
    type = local.has_ssl_certificate ? "redirect" : "forward"

    dynamic "redirect" {
      for_each = local.has_ssl_certificate ? [1] : []
      content {
        port        = "443"
        protocol    = "HTTPS"
        status_code = "HTTP_301"
      }
    }

    target_group_arn = local.has_ssl_certificate ? null : aws_lb_target_group.main.arn
  }
}

resource "aws_lb_listener" "https" {
  count = local.has_ssl_certificate ? 1 : 0

  load_balancer_arn = aws_lb.main.arn
  port              = 443
  protocol          = "HTTPS"
  ssl_policy        = "ELBSecurityPolicy-TLS13-1-2-2021-06"
  certificate_arn   = var.ssl_certificate_arn

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.main.arn
  }
}

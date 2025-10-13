resource "aws_iam_role" "ecs_task_execution" {
  name = "omni-${var.customer_name}-execution-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect = "Allow"
      Principal = {
        Service = "ecs-tasks.amazonaws.com"
      }
      Action = "sts:AssumeRole"
    }]
  })

  managed_policy_arns = [
    "arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy"
  ]

  inline_policy {
    name = "SecretsManagerAccess"
    policy = jsonencode({
      Version = "2012-10-17"
      Statement = [{
        Effect = "Allow"
        Action = [
          "secretsmanager:GetSecretValue"
        ]
        Resource = [
          var.database_password_arn,
          var.jina_api_key_arn,
          var.encryption_key_arn,
          var.encryption_salt_arn,
          var.session_secret_arn
        ]
      }]
    })
  }

  tags = merge(local.common_tags, {
    Name = "omni-${var.customer_name}-execution-role"
  })
}

resource "aws_iam_role" "ecs_task" {
  name = "omni-${var.customer_name}-task-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect = "Allow"
      Principal = {
        Service = "ecs-tasks.amazonaws.com"
      }
      Action = "sts:AssumeRole"
    }]
  })

  inline_policy {
    name = "BedrockAccess"
    policy = jsonencode({
      Version = "2012-10-17"
      Statement = [{
        Effect = "Allow"
        Action = [
          "bedrock:InvokeModel",
          "bedrock:InvokeModelWithResponseStream"
        ]
        Resource = [
          "arn:aws:bedrock:${var.region}:${data.aws_caller_identity.current.account_id}:inference-profile/eu.amazon.nova-pro-v1:0",
          "arn:aws:bedrock:${var.region}:${data.aws_caller_identity.current.account_id}:inference-profile/eu.amazon.nova-micro-v1:0",
          "arn:aws:bedrock:${var.region}:${data.aws_caller_identity.current.account_id}:inference-profile/eu.amazon.nova-lite-v1:0",
          "arn:aws:bedrock:*::foundation-model/amazon.nova-pro-v1:0",
          "arn:aws:bedrock:*::foundation-model/amazon.nova-micro-v1:0",
          "arn:aws:bedrock:*::foundation-model/amazon.nova-lite-v1:0",
          "arn:aws:bedrock:${var.region}:${data.aws_caller_identity.current.account_id}:inference-profile/eu.anthropic.claude-sonnet-4-20250514-v1:0",
          "arn:aws:bedrock:${var.region}:${data.aws_caller_identity.current.account_id}:inference-profile/eu.anthropic.claude-3-7-sonnet-20250219-v1:0",
          "arn:aws:bedrock:${var.region}:${data.aws_caller_identity.current.account_id}:inference-profile/eu.anthropic.claude-3-5-sonnet-20240620-v1:0",
          "arn:aws:bedrock:${var.region}:${data.aws_caller_identity.current.account_id}:inference-profile/eu.anthropic.claude-3-haiku-20240307-v1:0",
          "arn:aws:bedrock:*::foundation-model/anthropic.claude-sonnet-4-20250514-v1:0",
          "arn:aws:bedrock:*::foundation-model/anthropic.claude-3-7-sonnet-20250219-v1:0",
          "arn:aws:bedrock:*::foundation-model/anthropic.claude-3-5-sonnet-20240620-v1:0",
          "arn:aws:bedrock:*::foundation-model/anthropic.claude-3-haiku-20240307-v1:0"
        ]
      }]
    })
  }

  inline_policy {
    name = "ECSExecAccess"
    policy = jsonencode({
      Version = "2012-10-17"
      Statement = [
        {
          Effect = "Allow"
          Action = [
            "ssmmessages:CreateControlChannel",
            "ssmmessages:CreateDataChannel",
            "ssmmessages:OpenControlChannel",
            "ssmmessages:OpenDataChannel"
          ]
          Resource = "*"
        },
        {
          Effect = "Allow"
          Action = [
            "logs:CreateLogStream",
            "logs:DescribeLogStreams",
            "logs:PutLogEvents"
          ]
          Resource = "*"
        }
      ]
    })
  }

  tags = merge(local.common_tags, {
    Name = "omni-${var.customer_name}-task-role"
  })
}

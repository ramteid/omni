locals {
  common_tags = {
    Application = "Omni"
    Customer    = var.customer_name
    Environment = var.environment
    ManagedBy   = "Terraform"
  }
}

resource "aws_iam_role" "lambda_execution" {
  name = "omni-${var.customer_name}-migrator-lambda-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect = "Allow"
      Principal = {
        Service = "lambda.amazonaws.com"
      }
      Action = "sts:AssumeRole"
    }]
  })

  managed_policy_arns = [
    "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole"
  ]

  inline_policy {
    name = "RunECSTask"
    policy = jsonencode({
      Version = "2012-10-17"
      Statement = [
        {
          Effect = "Allow"
          Action = [
            "ecs:RunTask",
            "ecs:DescribeTasks"
          ]
          Resource = "*"
        },
        {
          Effect = "Allow"
          Action = [
            "iam:PassRole"
          ]
          Resource = [
            var.task_execution_role_arn,
            var.task_role_arn
          ]
        }
      ]
    })
  }

  tags = merge(local.common_tags, {
    Name = "omni-${var.customer_name}-migrator-lambda-role"
  })
}

resource "aws_lambda_function" "migrator" {
  filename      = "${path.module}/lambda/migrator.zip"
  function_name = "omni-${var.customer_name}-migrator"
  role          = aws_iam_role.lambda_execution.arn
  handler       = "index.handler"
  runtime       = "python3.11"
  timeout       = 600

  source_code_hash = data.archive_file.lambda_zip.output_base64sha256

  tags = merge(local.common_tags, {
    Name = "omni-${var.customer_name}-migrator"
  })
}

data "archive_file" "lambda_zip" {
  type        = "zip"
  output_path = "${path.module}/lambda/migrator.zip"

  source {
    content  = file("${path.module}/lambda/migrator.py")
    filename = "index.py"
  }
}

# Create a hash of all migration files to detect changes
data "external" "migrations_hash" {
  program = ["bash", "-c", <<-EOT
    if [ -d "${path.root}/../../../services/migrations" ]; then
      hash=$(find ${path.root}/../../../services/migrations -name "*.sql" -type f -exec sha256sum {} \; | sort | sha256sum | cut -d' ' -f1)
    else
      hash="no-migrations"
    fi
    echo "{\"hash\": \"$hash\"}"
  EOT
  ]
}

resource "null_resource" "run_migrations" {
  triggers = {
    # Only run when migrations change or when the task definition changes
    migrations_hash        = data.external.migrations_hash.result.hash
    task_definition_arn    = var.migrator_task_definition_arn
    lambda_code_hash       = aws_lambda_function.migrator.source_code_hash
  }

  provisioner "local-exec" {
    command = <<-EOT
      aws lambda invoke \
        --function-name ${aws_lambda_function.migrator.function_name} \
        --cli-binary-format raw-in-base64-out \
        --payload '${jsonencode({
          Cluster        = var.cluster_name
          TaskDefinition = var.migrator_task_definition_arn
          Subnets        = join(",", var.subnet_ids)
          SecurityGroups = var.security_group_id
        })}' \
        --region ${var.region} \
        response.json

      # Check if migration succeeded
      if ! grep -q '"statusCode": 200' response.json 2>/dev/null; then
        echo "Migration failed!"
        cat response.json
        rm -f response.json
        exit 1
      fi

      rm -f response.json
    EOT
  }

  depends_on = [aws_lambda_function.migrator]
}

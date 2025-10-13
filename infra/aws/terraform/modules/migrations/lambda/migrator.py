import boto3
import json
import time

def handler(event, context):
    print(f"Event: {json.dumps(event)}")

    try:
        ecs = boto3.client('ecs')

        # Get parameters from event
        cluster = event['Cluster']
        task_definition = event['TaskDefinition']
        subnets = event['Subnets'].split(',')
        security_groups = event['SecurityGroups']

        if isinstance(security_groups, str):
            security_groups = [security_groups]

        # Run the migration task
        response = ecs.run_task(
            cluster=cluster,
            taskDefinition=task_definition,
            launchType='FARGATE',
            networkConfiguration={
                'awsvpcConfiguration': {
                    'subnets': subnets,
                    'securityGroups': security_groups,
                    'assignPublicIp': 'DISABLED'
                }
            }
        )

        if not response['tasks']:
            raise Exception("Failed to start migration task")

        task_arn = response['tasks'][0]['taskArn']
        print(f"Started migration task: {task_arn}")

        # Wait for task completion
        waiter = ecs.get_waiter('tasks_stopped')
        waiter.wait(
            cluster=cluster,
            tasks=[task_arn],
            WaiterConfig={
                'Delay': 10,
                'MaxAttempts': 60  # 10 minutes max wait
            }
        )

        # Check task exit status
        task_status = ecs.describe_tasks(
            cluster=cluster,
            tasks=[task_arn]
        )

        task = task_status['tasks'][0]

        # Check container exit code
        for container in task['containers']:
            if container.get('exitCode', 0) != 0:
                raise Exception(f"Migration failed with exit code: {container.get('exitCode')}")

        print("Migration completed successfully")

        return {
            'statusCode': 200,
            'body': json.dumps({
                'message': 'Migration completed successfully',
                'taskArn': task_arn
            })
        }

    except Exception as e:
        print(f"Error: {str(e)}")
        return {
            'statusCode': 500,
            'body': json.dumps({
                'error': str(e)
            })
        }

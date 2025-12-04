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

        # Wait a few seconds to ensure the task started successfully
        time.sleep(10)

        # Check if task is running or has already completed
        task_status = ecs.describe_tasks(
            cluster=cluster,
            tasks=[task_arn]
        )

        if not task_status['tasks']:
            raise Exception(f"Task {task_arn} not found after starting")

        task = task_status['tasks'][0]
        last_status = task.get('lastStatus', 'UNKNOWN')

        print(f"Task {task_arn} status: {last_status}")

        # If task already stopped, check exit code
        if last_status == 'STOPPED':
            for container in task['containers']:
                exit_code = container.get('exitCode', 0)
                if exit_code != 0:
                    raise Exception(f"Migration task failed with exit code: {exit_code}")
            print("Migration task completed successfully")
        else:
            print(f"Migration task is {last_status}. It will complete asynchronously.")

        return {
            'statusCode': 200,
            'body': json.dumps({
                'message': 'Migration task started successfully',
                'taskArn': task_arn,
                'status': last_status
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

"""
Content storage client for accessing document content from S3
"""

import logging
import boto3
import os
from typing import Optional

logger = logging.getLogger(__name__)


class ContentStorage:
    """Client for fetching document content from S3"""

    def __init__(self, bucket: str, region: Optional[str] = None):
        self.bucket = bucket
        if region:
            self.s3_client = boto3.client('s3', region_name=region)
        else:
            self.s3_client = boto3.client('s3')
        logger.info(f"Initialized content storage client for bucket: {bucket}")

    async def get_text(self, key: str) -> str:
        """Fetch text content from S3"""
        import asyncio

        loop = asyncio.get_event_loop()
        response = await loop.run_in_executor(
            None,
            lambda: self.s3_client.get_object(Bucket=self.bucket, Key=key)
        )

        content = response['Body'].read().decode('utf-8')
        return content


def create_content_storage() -> ContentStorage:
    """Factory function to create content storage from environment variables"""
    storage_backend = os.getenv('STORAGE_BACKEND', 'local')

    if storage_backend == 's3':
        bucket = os.getenv('S3_BUCKET')
        if not bucket:
            raise ValueError("S3_BUCKET environment variable is required when STORAGE_BACKEND=s3")

        region = os.getenv('S3_REGION') or os.getenv('AWS_REGION')
        return ContentStorage(bucket, region)
    else:
        raise ValueError(f"Unsupported storage backend for AI service: {storage_backend}")

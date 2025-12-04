"""
Content storage client for accessing document content from S3 or PostgreSQL
"""

import logging
import boto3
import os
from typing import Optional
from db.connection import get_db_pool

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


class PostgresContentStorage:
    """Client for fetching document content from PostgreSQL"""

    def __init__(self):
        logger.info("Initialized PostgreSQL content storage client")

    async def get_text(self, key: str) -> str:
        """Fetch text content from PostgreSQL content_blobs table"""
        pool = await get_db_pool()

        async with pool.acquire() as conn:
            row = await conn.fetchrow(
                """
                SELECT content, storage_backend, storage_key
                FROM content_blobs
                WHERE id = $1
                """,
                key
            )

            if not row:
                raise ValueError(f"Content not found for key: {key}")

            storage_backend = row['storage_backend']

            if storage_backend == 'postgres':
                content_bytes = row['content']
                if content_bytes is None:
                    raise ValueError(f"Content is null for key: {key}")
                return content_bytes.decode('utf-8')

            elif storage_backend == 's3':
                storage_key = row['storage_key']
                if not storage_key:
                    raise ValueError(f"Storage key is null for S3-backed content: {key}")

                bucket = os.getenv('S3_BUCKET')
                if not bucket:
                    raise ValueError("S3_BUCKET environment variable is required for S3-backed content")

                region = os.getenv('S3_REGION') or os.getenv('AWS_REGION')
                if region:
                    s3_client = boto3.client('s3', region_name=region)
                else:
                    s3_client = boto3.client('s3')

                import asyncio
                loop = asyncio.get_event_loop()
                response = await loop.run_in_executor(
                    None,
                    lambda: s3_client.get_object(Bucket=bucket, Key=storage_key)
                )
                return response['Body'].read().decode('utf-8')

            else:
                raise ValueError(f"Unsupported storage backend in database: {storage_backend}")


def create_content_storage():
    """Factory function to create content storage from environment variables"""
    storage_backend = os.getenv('STORAGE_BACKEND', 'postgres')

    if storage_backend == 's3':
        bucket = os.getenv('S3_BUCKET')
        if not bucket:
            raise ValueError("S3_BUCKET environment variable is required when STORAGE_BACKEND=s3")

        region = os.getenv('S3_REGION') or os.getenv('AWS_REGION')
        return ContentStorage(bucket, region)

    elif storage_backend == 'postgres':
        return PostgresContentStorage()

    else:
        raise ValueError(f"Unsupported storage backend for AI service: {storage_backend}")

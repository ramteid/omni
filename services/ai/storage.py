"""
Content storage client for accessing document content from S3 or PostgreSQL
"""

import logging
import boto3
import os
from typing import Optional

from db.content_blobs import ContentBlobsRepository

logger = logging.getLogger(__name__)


class ContentStorage:
    """Client for fetching document content from S3"""

    def __init__(
        self,
        bucket: str,
        content_blobs_repo: ContentBlobsRepository,
        region: Optional[str] = None,
    ):
        self.bucket = bucket
        self.content_blobs_repo = content_blobs_repo
        if region:
            self.s3_client = boto3.client("s3", region_name=region)
        else:
            self.s3_client = boto3.client("s3")
        logger.info(f"Initialized content storage client for bucket: {bucket}")

    async def get_text(self, content_id: str) -> str:
        """Fetch text content by content_id. Looks up storage_key from DB and fetches from S3."""
        import asyncio

        blob = await self.content_blobs_repo.get_by_id(content_id)
        if not blob:
            raise ValueError(f"Content not found for id: {content_id}")
        if not blob.storage_key:
            raise ValueError(f"Storage key is null for content id: {content_id}")

        loop = asyncio.get_event_loop()
        response = await loop.run_in_executor(
            None,
            lambda: self.s3_client.get_object(Bucket=self.bucket, Key=blob.storage_key),
        )

        content = response["Body"].read().decode("utf-8")
        return content


class PostgresContentStorage:
    """Client for fetching document content from PostgreSQL"""

    def __init__(self, content_blobs_repo: ContentBlobsRepository):
        self.content_blobs_repo = content_blobs_repo
        logger.info("Initialized PostgreSQL content storage client")

    async def get_text(self, content_id: str) -> str:
        """Fetch text content by content_id from PostgreSQL content_blobs table"""
        blob = await self.content_blobs_repo.get_by_id(content_id)

        if not blob:
            raise ValueError(f"Content not found for id: {content_id}")

        if blob.storage_backend != "postgres":
            raise ValueError(
                f"Content {content_id} has storage_backend '{blob.storage_backend}', expected 'postgres'"
            )

        if blob.content is None:
            raise ValueError(f"Content is null for id: {content_id}")

        return blob.content.decode("utf-8")


def create_content_storage():
    """Factory function to create content storage from environment variables"""
    storage_backend = os.getenv("STORAGE_BACKEND", "postgres")
    content_blobs_repo = ContentBlobsRepository()

    if storage_backend == "s3":
        bucket = os.getenv("S3_BUCKET")
        if not bucket:
            raise ValueError(
                "S3_BUCKET environment variable is required when STORAGE_BACKEND=s3"
            )

        region = os.getenv("S3_REGION") or os.getenv("AWS_REGION")
        return ContentStorage(bucket, content_blobs_repo, region)

    elif storage_backend == "postgres":
        return PostgresContentStorage(content_blobs_repo)

    else:
        raise ValueError(
            f"Unsupported storage backend for AI service: {storage_backend}"
        )

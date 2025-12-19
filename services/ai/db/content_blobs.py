"""Repository for content blob database operations."""

import logging
from typing import Optional
from dataclasses import dataclass
from asyncpg import Pool

from .connection import get_db_pool

logger = logging.getLogger(__name__)


@dataclass
class ContentBlobRecord:
    """Content blob record from database"""

    id: str
    content: Optional[bytes]
    storage_key: Optional[str]
    storage_backend: str


class ContentBlobsRepository:
    """Repository for content blob database operations."""

    def __init__(self, pool: Optional[Pool] = None):
        self.pool = pool

    async def _get_pool(self) -> Pool:
        """Get database pool"""
        if self.pool:
            return self.pool
        return await get_db_pool()

    async def get_by_id(self, content_id: str) -> Optional[ContentBlobRecord]:
        """Get content blob by ID, including content and storage info."""
        pool = await self._get_pool()

        row = await pool.fetchrow(
            """
            SELECT id, content, storage_key, storage_backend
            FROM content_blobs
            WHERE id = $1
            """,
            content_id,
        )

        if row:
            return ContentBlobRecord(
                id=row["id"],
                content=row["content"],
                storage_key=row["storage_key"],
                storage_backend=row["storage_backend"],
            )
        return None

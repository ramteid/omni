"""Repository for embedding batch jobs database operations."""

import logging
from typing import Optional, List
from dataclasses import dataclass
from datetime import datetime
from ulid import ULID
from asyncpg import Pool

from .connection import get_db_pool

logger = logging.getLogger(__name__)


@dataclass
class BatchJob:
    """Represents a batch job record from database"""
    id: str
    status: str
    provider: str
    external_job_id: Optional[str]
    input_storage_path: Optional[str]
    output_storage_path: Optional[str]
    document_count: int
    created_at: datetime
    submitted_at: Optional[datetime]
    completed_at: Optional[datetime]
    error_message: Optional[str]


class EmbeddingBatchJobsRepository:
    """Repository for embedding batch jobs database operations."""

    def __init__(self, pool: Optional[Pool] = None):
        self.pool = pool

    async def _get_pool(self) -> Pool:
        """Get database pool"""
        if self.pool:
            return self.pool
        return await get_db_pool()

    async def create(self, provider: str) -> str:
        """Create new batch job record, return job_id"""
        pool = await self._get_pool()

        job_id = str(ULID())
        await pool.execute(
            """
            INSERT INTO embedding_batch_jobs (id, status, provider)
            VALUES ($1, 'pending', $2)
            """,
            job_id, provider
        )
        logger.info(f"Created batch job {job_id}")
        return job_id

    async def get_active(self) -> List[BatchJob]:
        """Get jobs in submitted/processing state"""
        pool = await self._get_pool()

        rows = await pool.fetch(
            """
            SELECT id, status, provider, external_job_id, input_storage_path,
                   output_storage_path, document_count, created_at, submitted_at,
                   completed_at, error_message
            FROM embedding_batch_jobs
            WHERE status IN ('submitted', 'processing')
            ORDER BY created_at ASC
            """
        )
        return [BatchJob(**dict(row)) for row in rows]

    async def get_by_id(self, job_id: str) -> Optional[BatchJob]:
        """Get a batch job by ID"""
        pool = await self._get_pool()

        row = await pool.fetchrow(
            """
            SELECT id, status, provider, external_job_id, input_storage_path,
                   output_storage_path, document_count, created_at, submitted_at,
                   completed_at, error_message
            FROM embedding_batch_jobs
            WHERE id = $1
            """,
            job_id
        )

        if row:
            return BatchJob(**dict(row))
        return None

    async def update_status(
        self,
        job_id: str,
        status: str,
        external_job_id: Optional[str] = None,
        input_storage_path: Optional[str] = None,
        output_storage_path: Optional[str] = None,
        error_message: Optional[str] = None,
        document_count: Optional[int] = None,
    ) -> None:
        """Update batch job status and other fields"""
        pool = await self._get_pool()

        updates = ['status = $2']
        params: list = [job_id, status]
        param_idx = 3

        if external_job_id is not None:
            updates.append(f'external_job_id = ${param_idx}')
            params.append(external_job_id)
            param_idx += 1

        if input_storage_path is not None:
            updates.append(f'input_storage_path = ${param_idx}')
            params.append(input_storage_path)
            param_idx += 1

        if output_storage_path is not None:
            updates.append(f'output_storage_path = ${param_idx}')
            params.append(output_storage_path)
            param_idx += 1

        if error_message is not None:
            updates.append(f'error_message = ${param_idx}')
            params.append(error_message)
            param_idx += 1

        if document_count is not None:
            updates.append(f'document_count = ${param_idx}')
            params.append(document_count)
            param_idx += 1

        if status == 'submitted':
            updates.append('submitted_at = CURRENT_TIMESTAMP')
        elif status in ('completed', 'failed'):
            updates.append('completed_at = CURRENT_TIMESTAMP')

        query = f"""
            UPDATE embedding_batch_jobs
            SET {', '.join(updates)}
            WHERE id = $1
        """

        await pool.execute(query, *params)
        logger.info(f"Updated batch job {job_id} to status: {status}")

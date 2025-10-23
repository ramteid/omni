"""
Handles batch inference for embeddings using cloud providers (AWS Bedrock, etc.)
Manages the complete lifecycle: accumulation → submission → monitoring → result processing
"""

import asyncio
import logging
import json
import time
import boto3
from typing import List, Dict, Optional, Tuple, Any
from dataclasses import dataclass
from collections import defaultdict
from datetime import datetime
import ulid
import asyncpg

from ..config import (
    DATABASE_URL,
    ENABLE_EMBEDDING_BATCH_INFERENCE,
    EMBEDDING_BATCH_S3_BUCKET,
    EMBEDDING_BATCH_BEDROCK_ROLE_ARN,
    EMBEDDING_BATCH_MIN_DOCUMENTS,
    EMBEDDING_BATCH_MAX_DOCUMENTS,
    EMBEDDING_BATCH_ACCUMULATION_TIMEOUT_SECONDS,
    EMBEDDING_BATCH_ACCUMULATION_POLL_INTERVAL,
    EMBEDDING_BATCH_MONITOR_POLL_INTERVAL,
    BEDROCK_EMBEDDING_MODEL_ID,
    AWS_REGION,
)
from embeddings import chunk_by_sentences

logger = logging.getLogger(__name__)


# ============================================================================
# Data Models
# ============================================================================
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


@dataclass
class EmbeddingQueueItem:
    """Represents an embedding_queue item"""
    id: str
    document_id: str
    status: str
    batch_job_id: Optional[str]
    created_at: datetime


# ============================================================================
# Storage Operations (Cloud-Agnostic Interface)
# ============================================================================
class StorageClient:
    """Abstraction for cloud storage operations (S3, GCS, etc.)"""

    async def upload_jsonl(self, path: str, records: List[dict]) -> None:
        """Upload JSONL to cloud storage"""
        raise NotImplementedError

    async def download_jsonl(self, path: str) -> List[dict]:
        """Download JSONL from cloud storage"""
        raise NotImplementedError

    async def list_files(self, prefix: str) -> List[str]:
        """List files with given prefix"""
        raise NotImplementedError


class S3StorageClient(StorageClient):
    """S3-specific implementation"""

    def __init__(self, bucket: str, region: Optional[str] = None):
        self.bucket = bucket
        if region:
            self.s3_client = boto3.client('s3', region_name=region)
        else:
            self.s3_client = boto3.client('s3')
        logger.info(f"Initialized S3 storage client for bucket: {bucket}")

    async def upload_jsonl(self, path: str, records: List[dict]) -> None:
        """Upload JSONL to S3"""
        # Convert records to JSONL format
        jsonl_content = '\n'.join(json.dumps(record) for record in records)

        # Upload to S3
        loop = asyncio.get_event_loop()
        await loop.run_in_executor(
            None,
            lambda: self.s3_client.put_object(
                Bucket=self.bucket,
                Key=path,
                Body=jsonl_content.encode('utf-8')
            )
        )
        logger.info(f"Uploaded {len(records)} records to s3://{self.bucket}/{path}")

    async def download_jsonl(self, path: str) -> List[dict]:
        """Download JSONL from S3"""
        loop = asyncio.get_event_loop()
        response = await loop.run_in_executor(
            None,
            lambda: self.s3_client.get_object(Bucket=self.bucket, Key=path)
        )

        content = response['Body'].read().decode('utf-8')
        records = [json.loads(line) for line in content.strip().split('\n') if line.strip()]
        logger.info(f"Downloaded {len(records)} records from s3://{self.bucket}/{path}")
        return records

    async def list_files(self, prefix: str) -> List[str]:
        """List files with given prefix in S3"""
        loop = asyncio.get_event_loop()
        response = await loop.run_in_executor(
            None,
            lambda: self.s3_client.list_objects_v2(Bucket=self.bucket, Prefix=prefix)
        )

        files = []
        if 'Contents' in response:
            files = [obj['Key'] for obj in response['Contents']]

        logger.debug(f"Found {len(files)} files with prefix: {prefix}")
        return files


# ============================================================================
# Provider Interface (Cloud-Agnostic)
# ============================================================================
class BatchInferenceProvider:
    """Abstract interface for batch inference providers"""

    async def submit_job(
        self,
        input_path: str,
        output_path: str,
        job_name: str
    ) -> str:
        """Submit batch job, return external_job_id"""
        raise NotImplementedError

    async def get_job_status(self, external_job_id: str) -> Tuple[str, Optional[str]]:
        """Get job status, return (status, error_message)"""
        raise NotImplementedError


class BedrockBatchProvider(BatchInferenceProvider):
    """AWS Bedrock batch inference implementation"""

    def __init__(self, model_id: str, role_arn: str, region: Optional[str] = None):
        self.model_id = model_id
        self.role_arn = role_arn
        if region:
            self.bedrock_client = boto3.client('bedrock', region_name=region)
        else:
            self.bedrock_client = boto3.client('bedrock')
        logger.info(f"Initialized Bedrock batch provider with model: {model_id}")

    async def submit_job(
        self,
        input_path: str,
        output_path: str,
        job_name: str
    ) -> str:
        """Submit batch job to Bedrock"""
        loop = asyncio.get_event_loop()

        response = await loop.run_in_executor(
            None,
            lambda: self.bedrock_client.create_model_invocation_job(
                roleArn=self.role_arn,
                modelId=self.model_id,
                jobName=job_name,
                inputDataConfig={
                    's3InputDataConfig': {
                        's3Uri': input_path
                    }
                },
                outputDataConfig={
                    's3OutputDataConfig': {
                        's3Uri': output_path
                    }
                }
            )
        )

        job_arn = response['jobArn']
        logger.info(f"Submitted Bedrock batch job: {job_arn}")
        return job_arn

    async def get_job_status(self, external_job_id: str) -> Tuple[str, Optional[str]]:
        """Get job status from Bedrock"""
        loop = asyncio.get_event_loop()

        try:
            response = await loop.run_in_executor(
                None,
                lambda: self.bedrock_client.get_model_invocation_job(
                    jobIdentifier=external_job_id
                )
            )

            status = response['status']
            error_message = response.get('message') if status == 'Failed' else None

            # Map Bedrock status to our internal status
            status_map = {
                'Submitted': 'submitted',
                'InProgress': 'processing',
                'Completed': 'completed',
                'Failed': 'failed',
                'Stopping': 'processing',
                'Stopped': 'failed'
            }

            internal_status = status_map.get(status, 'processing')
            return (internal_status, error_message)

        except Exception as e:
            logger.error(f"Error getting job status for {external_job_id}: {e}")
            return ('failed', str(e))


# ============================================================================
# Database Operations
# ============================================================================
class BatchDatabase:
    """Database operations for batch processing"""

    def __init__(self, pool: asyncpg.Pool):
        self.pool = pool

    async def get_pending_queue_items(self, limit: int) -> List[EmbeddingQueueItem]:
        """Fetch pending items not assigned to any batch"""
        rows = await self.pool.fetch(
            """
            SELECT id, document_id, status, batch_job_id, created_at
            FROM embedding_queue
            WHERE status = 'pending' AND batch_job_id IS NULL
            ORDER BY created_at ASC
            LIMIT $1
            """,
            limit
        )
        return [EmbeddingQueueItem(**dict(row)) for row in rows]

    async def create_batch_job(self, provider: str, document_count: int) -> str:
        """Create new batch job record, return job_id"""
        job_id = str(ulid.ULID())
        await self.pool.execute(
            """
            INSERT INTO embedding_batch_jobs (id, status, provider, document_count)
            VALUES ($1, 'pending', $2, $3)
            """,
            job_id, provider, document_count
        )
        logger.info(f"Created batch job {job_id} with {document_count} documents")
        return job_id

    async def assign_items_to_batch(self, batch_id: str, item_ids: List[str]) -> None:
        """Assign queue items to batch job"""
        await self.pool.execute(
            """
            UPDATE embedding_queue
            SET batch_job_id = $1, status = 'batched'
            WHERE id = ANY($2)
            """,
            batch_id, item_ids
        )
        logger.info(f"Assigned {len(item_ids)} items to batch {batch_id}")

    async def get_active_batch_jobs(self) -> List[BatchJob]:
        """Get jobs in submitted/processing state"""
        rows = await self.pool.fetch(
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

    async def update_batch_job_status(
        self,
        job_id: str,
        status: str,
        external_job_id: Optional[str] = None,
        input_storage_path: Optional[str] = None,
        output_storage_path: Optional[str] = None,
        error_message: Optional[str] = None
    ) -> None:
        """Update batch job status and other fields"""
        updates = ['status = $2']
        params = [job_id, status]
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

        if status == 'submitted':
            updates.append('submitted_at = CURRENT_TIMESTAMP')
        elif status in ('completed', 'failed'):
            updates.append('completed_at = CURRENT_TIMESTAMP')

        query = f"""
            UPDATE embedding_batch_jobs
            SET {', '.join(updates)}
            WHERE id = $1
        """

        await self.pool.execute(query, *params)
        logger.info(f"Updated batch job {job_id} to status: {status}")

    async def get_queue_items_for_batch(self, batch_id: str) -> List[EmbeddingQueueItem]:
        """Get all queue items for a batch"""
        rows = await self.pool.fetch(
            """
            SELECT id, document_id, status, batch_job_id, created_at
            FROM embedding_queue
            WHERE batch_job_id = $1
            ORDER BY created_at ASC
            """,
            batch_id
        )
        return [EmbeddingQueueItem(**dict(row)) for row in rows]

    async def mark_items_completed(self, item_ids: List[str]) -> None:
        """Mark queue items as completed"""
        await self.pool.execute(
            """
            UPDATE embedding_queue
            SET status = 'completed', processed_at = CURRENT_TIMESTAMP
            WHERE id = ANY($1)
            """,
            item_ids
        )
        logger.info(f"Marked {len(item_ids)} queue items as completed")

    async def mark_items_failed(self, item_ids: List[str], error: str) -> None:
        """Mark queue items as failed"""
        await self.pool.execute(
            """
            UPDATE embedding_queue
            SET status = 'failed', error_message = $2, processed_at = CURRENT_TIMESTAMP
            WHERE id = ANY($1)
            """,
            item_ids, error
        )
        logger.error(f"Marked {len(item_ids)} queue items as failed: {error}")

    async def update_documents_embedding_status(
        self,
        document_ids: List[str],
        status: str
    ) -> None:
        """Update embedding_status for documents"""
        await self.pool.execute(
            """
            UPDATE documents
            SET embedding_status = $2
            WHERE id = ANY($1)
            """,
            document_ids, status
        )
        logger.info(f"Updated {len(document_ids)} documents to embedding_status: {status}")

    async def bulk_insert_embeddings(self, embeddings: List[Dict[str, Any]]) -> None:
        """Bulk insert embeddings into database"""
        if not embeddings:
            return

        # Prepare data for COPY
        records = [
            (
                emb['id'],
                emb['document_id'],
                emb['chunk_index'],
                emb['chunk_start_offset'],
                emb['chunk_end_offset'],
                emb['embedding'],
                emb['model_name'],
                emb.get('created_at', datetime.utcnow())
            )
            for emb in embeddings
        ]

        # Use COPY for efficient bulk insert
        await self.pool.copy_records_to_table(
            'embeddings',
            records=records,
            columns=[
                'id', 'document_id', 'chunk_index', 'chunk_start_offset',
                'chunk_end_offset', 'embedding', 'model_name', 'created_at'
            ]
        )
        logger.info(f"Bulk inserted {len(embeddings)} embeddings")

    async def delete_embeddings_for_documents(self, document_ids: List[str]) -> None:
        """Delete existing embeddings for documents (before reinserting)"""
        result = await self.pool.execute(
            """
            DELETE FROM embeddings
            WHERE document_id = ANY($1)
            """,
            document_ids
        )
        logger.info(f"Deleted existing embeddings for {len(document_ids)} documents")


# ============================================================================
# Batch Processing Logic
# ============================================================================
class EmbeddingBatchProcessor:
    """Main class that orchestrates batch processing"""

    def __init__(self, db_pool: asyncpg.Pool, content_storage, embedding_provider):
        self.db = BatchDatabase(db_pool)
        self.content_storage = content_storage
        self.storage_client = self._create_storage_client()
        self.provider = self._create_provider()
        self.embedding_provider = embedding_provider  # For chunking logic

        self.accumulation_state = {
            'last_seen_count': 0,
            'last_change_time': time.time()
        }

    def _create_storage_client(self) -> StorageClient:
        """Factory for storage client based on config"""
        if EMBEDDING_BATCH_S3_BUCKET:
            return S3StorageClient(EMBEDDING_BATCH_S3_BUCKET, AWS_REGION)
        raise ValueError("EMBEDDING_BATCH_S3_BUCKET must be configured")

    def _create_provider(self) -> BatchInferenceProvider:
        """Factory for batch provider based on config"""
        # Currently only Bedrock is supported
        if not BEDROCK_EMBEDDING_MODEL_ID or not EMBEDDING_BATCH_BEDROCK_ROLE_ARN:
            raise ValueError("BEDROCK_EMBEDDING_MODEL_ID and EMBEDDING_BATCH_BEDROCK_ROLE_ARN must be configured")

        return BedrockBatchProvider(
            BEDROCK_EMBEDDING_MODEL_ID,
            EMBEDDING_BATCH_BEDROCK_ROLE_ARN,
            AWS_REGION
        )

    # ------------------------------------------------------------------------
    # Accumulation Loop
    # ------------------------------------------------------------------------
    async def accumulation_loop(self):
        """Background task: accumulate items and trigger batch creation"""
        logger.info("Starting embedding batch accumulation loop")

        while True:
            try:
                await self._check_and_create_batch()
                await asyncio.sleep(EMBEDDING_BATCH_ACCUMULATION_POLL_INTERVAL)
            except Exception as e:
                logger.error(f"Accumulation loop error: {e}", exc_info=True)
                await asyncio.sleep(10)

    async def _check_and_create_batch(self):
        """Check thresholds and create batch if conditions met"""
        # Query pending items
        items = await self.db.get_pending_queue_items(EMBEDDING_BATCH_MAX_DOCUMENTS)
        current_count = len(items)

        if current_count == 0:
            return

        # Update accumulation state
        current_time = time.time()
        if current_count != self.accumulation_state['last_seen_count']:
            self.accumulation_state['last_seen_count'] = current_count
            self.accumulation_state['last_change_time'] = current_time

        time_since_last_change = current_time - self.accumulation_state['last_change_time']

        # Check thresholds
        should_create_batch = (
            current_count >= EMBEDDING_BATCH_MIN_DOCUMENTS and
            (
                current_count >= EMBEDDING_BATCH_MAX_DOCUMENTS or
                time_since_last_change >= EMBEDDING_BATCH_ACCUMULATION_TIMEOUT_SECONDS
            )
        )

        if should_create_batch:
            logger.info(
                f"Creating batch with {current_count} documents "
                f"(idle for {time_since_last_change:.1f}s)"
            )
            await self.create_batch(items)

            # Reset accumulation state
            self.accumulation_state['last_seen_count'] = 0
            self.accumulation_state['last_change_time'] = current_time

    async def create_batch(self, items: List[EmbeddingQueueItem]):
        """Create and submit a new batch job"""
        try:
            # Create batch job record
            batch_id = await self.db.create_batch_job('bedrock', len(items))

            # Assign items to batch
            item_ids = [item.id for item in items]
            await self.db.assign_items_to_batch(batch_id, item_ids)

            # Prepare and submit in background
            asyncio.create_task(self._prepare_and_submit_safe(batch_id))

        except Exception as e:
            logger.error(f"Failed to create batch: {e}", exc_info=True)

    async def _prepare_and_submit_safe(self, batch_id: str):
        """Safe wrapper for prepare_and_submit with error handling"""
        try:
            await self.prepare_and_submit(batch_id)
        except Exception as e:
            logger.error(f"Failed to prepare/submit batch {batch_id}: {e}", exc_info=True)

            # Mark batch and items as failed
            await self.db.update_batch_job_status(batch_id, 'failed', error_message=str(e))
            items = await self.db.get_queue_items_for_batch(batch_id)
            await self.db.mark_items_failed([item.id for item in items], str(e))

    # ------------------------------------------------------------------------
    # Batch Preparation & Submission
    # ------------------------------------------------------------------------
    async def prepare_and_submit(self, batch_id: str):
        """Prepare JSONL and submit to cloud provider"""
        logger.info(f"Preparing batch {batch_id}")

        # Update status
        await self.db.update_batch_job_status(batch_id, 'preparing')

        # Fetch items for this batch
        items = await self.db.get_queue_items_for_batch(batch_id)

        # Fetch documents and chunk content
        jsonl_records = await self._fetch_and_chunk_documents(items)

        if not jsonl_records:
            raise ValueError(f"No valid records generated for batch {batch_id}")

        # Upload to storage
        input_key = f"input/{batch_id}.jsonl"
        input_path = f"s3://{EMBEDDING_BATCH_S3_BUCKET}/{input_key}"
        await self.storage_client.upload_jsonl(input_key, jsonl_records)

        # Submit job to provider
        output_path = f"s3://{EMBEDDING_BATCH_S3_BUCKET}/output/{batch_id}/"
        job_name = f"embedding-batch-{batch_id}"
        external_job_id = await self.provider.submit_job(input_path, output_path, job_name)

        # Update batch job with submission details
        await self.db.update_batch_job_status(
            batch_id,
            'submitted',
            external_job_id=external_job_id,
            input_storage_path=input_path,
            output_storage_path=output_path
        )

        # Update queue items to processing
        await self.db.pool.execute(
            """
            UPDATE embedding_queue
            SET status = 'processing'
            WHERE batch_job_id = $1
            """,
            batch_id
        )

        logger.info(f"Batch {batch_id} submitted successfully (external_job_id: {external_job_id})")

    async def _fetch_and_chunk_documents(self, items: List[EmbeddingQueueItem]) -> List[dict]:
        """Fetch content and generate JSONL records"""
        from embeddings import chunk_by_sentences

        jsonl_records = []

        for item in items:
            try:
                # Fetch document from DB
                doc_row = await self.db.pool.fetchrow(
                    "SELECT id, content_id FROM documents WHERE id = $1",
                    item.document_id
                )

                if not doc_row or not doc_row['content_id']:
                    logger.warning(f"Document {item.document_id} has no content_id, skipping")
                    continue

                # Fetch content from storage
                content = await self.content_storage.get_text(doc_row['content_id'])

                if not content or not content.strip():
                    logger.warning(f"Document {item.document_id} has no content, skipping")
                    continue

                # Chunk content using existing logic
                chunk_spans = chunk_by_sentences(content, chunk_size=512)

                # Create JSONL records for each chunk
                for chunk_idx, (start, end) in enumerate(chunk_spans):
                    chunk_text = content[start:end]

                    jsonl_records.append({
                        'recordId': f"{item.document_id}:{chunk_idx}:{start}:{end}",
                        'modelInput': {
                            'inputText': chunk_text
                        }
                    })

                logger.debug(
                    f"Document {item.document_id} chunked into {len(chunk_spans)} chunks"
                )

            except Exception as e:
                logger.error(
                    f"Failed to process document {item.document_id}: {e}",
                    exc_info=True
                )
                continue

        logger.info(f"Generated {len(jsonl_records)} JSONL records from {len(items)} documents")
        return jsonl_records

    # ------------------------------------------------------------------------
    # Monitoring Loop
    # ------------------------------------------------------------------------
    async def monitoring_loop(self):
        """Background task: poll job status and process results"""
        logger.info("Starting embedding batch monitoring loop")

        while True:
            try:
                await self._monitor_active_jobs()
                await asyncio.sleep(EMBEDDING_BATCH_MONITOR_POLL_INTERVAL)
            except Exception as e:
                logger.error(f"Monitoring loop error: {e}", exc_info=True)
                await asyncio.sleep(30)

    async def _monitor_active_jobs(self):
        """Check status of all active jobs"""
        jobs = await self.db.get_active_batch_jobs()

        if not jobs:
            return

        logger.debug(f"Monitoring {len(jobs)} active batch jobs")

        for job in jobs:
            try:
                # Poll provider for job status
                status, error_message = await self.provider.get_job_status(job.external_job_id)

                if status == 'completed':
                    logger.info(f"Batch job {job.id} completed, processing results")
                    await self.process_results(job.id)

                elif status == 'failed':
                    logger.error(f"Batch job {job.id} failed: {error_message}")
                    await self._mark_batch_failed(job.id, error_message)

                elif status != job.status:
                    # Update status if changed
                    await self.db.update_batch_job_status(job.id, status)

            except Exception as e:
                logger.error(f"Error monitoring job {job.id}: {e}", exc_info=True)

    async def _mark_batch_failed(self, batch_id: str, error_message: Optional[str]):
        """Mark batch job and all its items as failed"""
        await self.db.update_batch_job_status(batch_id, 'failed', error_message=error_message)

        items = await self.db.get_queue_items_for_batch(batch_id)
        await self.db.mark_items_failed(
            [item.id for item in items],
            error_message or "Batch job failed"
        )

    # ------------------------------------------------------------------------
    # Result Processing
    # ------------------------------------------------------------------------
    async def process_results(self, batch_id: str):
        """Download and process batch results"""
        logger.info(f"Processing results for batch {batch_id}")

        try:
            # Get batch job info
            job_row = await self.db.pool.fetchrow(
                "SELECT output_storage_path FROM embedding_batch_jobs WHERE id = $1",
                batch_id
            )

            if not job_row or not job_row['output_storage_path']:
                raise ValueError(f"No output path for batch {batch_id}")

            # Extract prefix from output path (e.g., "output/batch_id/")
            output_path = job_row['output_storage_path']
            # Remove s3://bucket/ prefix to get the key prefix
            prefix = output_path.replace(f"s3://{EMBEDDING_BATCH_S3_BUCKET}/", "")

            # List output files
            output_files = await self.storage_client.list_files(prefix)

            if not output_files:
                raise ValueError(f"No output files found for batch {batch_id}")

            # Download and parse all output files
            all_output_lines = []
            for file_key in output_files:
                if file_key.endswith('.jsonl') or file_key.endswith('.out'):
                    lines = await self.storage_client.download_jsonl(file_key)
                    all_output_lines.extend(lines)

            logger.info(f"Downloaded {len(all_output_lines)} output records for batch {batch_id}")

            # Parse and group embeddings by document
            embeddings_by_doc = await self._parse_and_group_embeddings(all_output_lines)

            # Store embeddings in database
            await self._store_embeddings(embeddings_by_doc)

            # Mark queue items as completed
            items = await self.db.get_queue_items_for_batch(batch_id)
            await self.db.mark_items_completed([item.id for item in items])

            # Update documents embedding_status
            document_ids = list(embeddings_by_doc.keys())
            await self.db.update_documents_embedding_status(document_ids, 'completed')

            # Mark batch job as completed
            await self.db.update_batch_job_status(batch_id, 'completed')

            logger.info(f"Successfully processed batch {batch_id} with {len(document_ids)} documents")

        except Exception as e:
            logger.error(f"Failed to process results for batch {batch_id}: {e}", exc_info=True)
            await self._mark_batch_failed(batch_id, str(e))

    async def _parse_and_group_embeddings(self, output_lines: List[dict]) -> Dict[str, List]:
        """Parse JSONL output and group by document_id"""
        embeddings_by_doc = defaultdict(list)

        for line in output_lines:
            try:
                record_id = line['recordId']  # "doc_id:chunk_idx:start:end"

                # Check for errors
                if 'error' in line:
                    logger.warning(f"Embedding error for {record_id}: {line['error']}")
                    continue

                # Extract embedding
                model_output = line.get('modelOutput', {})
                embedding = model_output.get('embedding')

                if not embedding:
                    logger.warning(f"No embedding in output for {record_id}")
                    continue

                # Parse record_id
                parts = record_id.split(':')
                if len(parts) != 4:
                    logger.warning(f"Invalid record_id format: {record_id}")
                    continue

                doc_id, chunk_idx, start, end = parts

                embeddings_by_doc[doc_id].append({
                    'chunk_index': int(chunk_idx),
                    'start': int(start),
                    'end': int(end),
                    'embedding': embedding
                })

            except Exception as e:
                logger.error(f"Error parsing output line: {e}", exc_info=True)
                continue

        # Sort chunks by index for each document
        for doc_id in embeddings_by_doc:
            embeddings_by_doc[doc_id].sort(key=lambda x: x['chunk_index'])

        logger.info(f"Grouped embeddings for {len(embeddings_by_doc)} documents")
        return embeddings_by_doc

    async def _store_embeddings(self, embeddings_by_doc: Dict[str, List]):
        """Bulk insert embeddings into database"""
        if not embeddings_by_doc:
            return

        # Delete existing embeddings for these documents
        document_ids = list(embeddings_by_doc.keys())
        await self.db.delete_embeddings_for_documents(document_ids)

        # Prepare embeddings for bulk insert
        all_embeddings = []
        for doc_id, chunks in embeddings_by_doc.items():
            for chunk in chunks:
                all_embeddings.append({
                    'id': str(ulid.ULID()),
                    'document_id': doc_id,
                    'chunk_index': chunk['chunk_index'],
                    'chunk_start_offset': chunk['start'],
                    'chunk_end_offset': chunk['end'],
                    'embedding': chunk['embedding'],
                    'model_name': BEDROCK_EMBEDDING_MODEL_ID
                })

        # Bulk insert
        await self.db.bulk_insert_embeddings(all_embeddings)

        logger.info(f"Stored {len(all_embeddings)} embeddings for {len(document_ids)} documents")


# ============================================================================
# Public API for Integration
# ============================================================================
async def start_batch_processing(content_storage, embedding_provider):
    """Start batch processing background tasks"""
    if not ENABLE_EMBEDDING_BATCH_INFERENCE:
        logger.info("Embedding batch inference is disabled")
        return

    logger.info("Starting embedding batch processing")

    # Create database connection pool for batch processor
    db_pool = await asyncpg.create_pool(
        DATABASE_URL,
        min_size=2,
        max_size=10,
        command_timeout=60
    )
    logger.info("Database connection pool created for batch processing")

    processor = EmbeddingBatchProcessor(db_pool, content_storage, embedding_provider)

    # Start both loops concurrently
    await asyncio.gather(
        processor.accumulation_loop(),
        processor.monitoring_loop()
    )

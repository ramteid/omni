"""
Database repository for configuration management.
Handles LLM and embedding configuration storage and retrieval.
"""
import logging
import json
from typing import Optional
from .connection import get_db_pool

logger = logging.getLogger(__name__)


async def fetch_llm_config() -> Optional[dict]:
    """Fetch LLM configuration from database"""
    try:
        pool = await get_db_pool()
        async with pool.acquire() as conn:
            row = await conn.fetchrow(
                """
                SELECT value
                FROM configuration
                WHERE key = 'llm_config'
                LIMIT 1
                """
            )

            if row and row['value']:
                value = row['value']
                if isinstance(value, str):
                    return json.loads(value)
                return value
            return None
    except Exception as e:
        logger.warning(f"Failed to fetch LLM config from database: {e}")
        return None


async def fetch_embedding_config() -> Optional[dict]:
    """Fetch embedding configuration from database"""
    try:
        pool = await get_db_pool()
        async with pool.acquire() as conn:
            row = await conn.fetchrow(
                """
                SELECT value
                FROM configuration
                WHERE key = 'embedding_config'
                LIMIT 1
                """
            )

            if row and row['value']:
                value = row['value']
                if isinstance(value, str):
                    return json.loads(value)
                return value
            return None
    except Exception as e:
        logger.warning(f"Failed to fetch embedding config from database: {e}")
        return None

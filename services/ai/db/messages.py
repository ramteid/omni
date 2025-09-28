from typing import Dict, Any, Optional, List
from datetime import datetime
from ulid import ULID
import asyncpg
from asyncpg import Pool
import json

from .models import ChatMessage
from .connection import get_db_pool

class MessagesRepository:
    def __init__(self, pool: Optional[Pool] = None):
        self.pool = pool

    async def _get_pool(self) -> Pool:
        """Get database pool"""
        if self.pool:
            return self.pool
        return await get_db_pool()

    async def create(
        self,
        chat_id: str,
        message: Dict[str, Any]
    ) -> ChatMessage:
        """Create a new message in a chat"""
        pool = await self._get_pool()

        message_id = str(ULID())

        # Get the next sequence number for this chat
        seq_query = """
            SELECT COALESCE(MAX(message_seq_num), 0) + 1 as next_seq
            FROM chat_messages
            WHERE chat_id = $1
        """

        async with pool.acquire() as conn:
            next_seq = await conn.fetchval(seq_query, chat_id)

            query = """
                INSERT INTO chat_messages (id, chat_id, message_seq_num, message, created_at)
                VALUES ($1, $2, $3, $4, NOW())
                RETURNING id, chat_id, message_seq_num, message, created_at
            """

            row = await conn.fetchrow(
                query,
                message_id,
                chat_id,
                next_seq,
                json.dumps(message)
            )

        return ChatMessage.from_row(dict(row))

    async def get_by_chat(self, chat_id: str) -> List[ChatMessage]:
        """Get all messages for a chat"""
        pool = await self._get_pool()

        query = """
            SELECT id, chat_id, message_seq_num, message, created_at
            FROM chat_messages
            WHERE chat_id = $1
            ORDER BY message_seq_num
        """

        async with pool.acquire() as conn:
            rows = await conn.fetch(query, chat_id)

        return [ChatMessage.from_row(dict(row)) for row in rows]
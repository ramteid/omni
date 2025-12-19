"""
Database-backed configuration for LLM and embedding settings.
Provides cached access to configuration stored in PostgreSQL,
with fallback to environment variables.
"""

import time
from typing import Optional
from dataclasses import dataclass

from config import (
    LLM_PROVIDER,
    VLLM_URL,
    ANTHROPIC_API_KEY,
    ANTHROPIC_MODEL,
    BEDROCK_MODEL_ID,
    TITLE_GENERATION_MODEL_ID,
    DEFAULT_MAX_TOKENS,
    DEFAULT_TEMPERATURE,
    DEFAULT_TOP_P,
    EMBEDDING_PROVIDER,
    JINA_API_KEY,
    JINA_MODEL,
    JINA_API_URL,
    BEDROCK_EMBEDDING_MODEL_ID,
    AWS_REGION,
)
from db import fetch_llm_config, fetch_embedding_config


@dataclass
class LLMConfig:
    """LLM configuration data class"""

    provider: str
    primary_model_id: str
    secondary_model_id: Optional[str]
    vllm_url: Optional[str]
    anthropic_api_key: Optional[str]
    max_tokens: Optional[int]
    temperature: Optional[float]
    top_p: Optional[float]


class LLMConfigCache:
    """
    Cached LLM configuration reader with PostgreSQL backend.
    Cache has a TTL of 90 seconds (1.5 minutes) to balance freshness with performance.
    """

    CACHE_TTL_SECONDS = 90  # 1.5 minutes

    def __init__(self):
        self._cache: Optional[LLMConfig] = None
        self._cache_timestamp: float = 0

    def _is_cache_valid(self) -> bool:
        """Check if cache is still valid"""
        if self._cache is None:
            return False
        elapsed = time.time() - self._cache_timestamp
        return elapsed < self.CACHE_TTL_SECONDS

    async def _fetch_from_database(self) -> Optional[LLMConfig]:
        """Fetch LLM configuration from database"""
        config_data = await fetch_llm_config()

        if config_data:
            return LLMConfig(
                provider=config_data.get("provider"),
                primary_model_id=config_data.get("primaryModelId"),
                secondary_model_id=config_data.get("secondaryModelId"),
                vllm_url=config_data.get("vllmUrl"),
                anthropic_api_key=config_data.get("anthropicApiKey"),
                max_tokens=config_data.get("maxTokens"),
                temperature=config_data.get("temperature"),
                top_p=config_data.get("topP"),
            )
        return None

    def _get_env_fallback_config(self) -> LLMConfig:
        """Get configuration from environment variables as fallback"""
        # Determine primary and secondary model IDs based on provider
        primary_model_id = ""
        secondary_model_id = None

        if LLM_PROVIDER == "anthropic":
            primary_model_id = ANTHROPIC_MODEL
        elif LLM_PROVIDER == "bedrock":
            primary_model_id = BEDROCK_MODEL_ID
            secondary_model_id = TITLE_GENERATION_MODEL_ID
        elif LLM_PROVIDER == "vllm":
            # For vLLM, there's no default model ID in env
            primary_model_id = "default"

        return LLMConfig(
            provider=LLM_PROVIDER,
            primary_model_id=primary_model_id,
            secondary_model_id=secondary_model_id,
            vllm_url=VLLM_URL if VLLM_URL else None,
            anthropic_api_key=ANTHROPIC_API_KEY if ANTHROPIC_API_KEY else None,
            max_tokens=DEFAULT_MAX_TOKENS,
            temperature=DEFAULT_TEMPERATURE,
            top_p=DEFAULT_TOP_P,
        )

    async def get_config(self) -> LLMConfig:
        """
        Get LLM configuration with caching.
        Priority: Database config -> Environment variables
        """
        # Return cached config if still valid
        if self._is_cache_valid():
            return self._cache  # type: ignore

        # Try to fetch from database
        db_config = await self._fetch_from_database()

        if db_config is not None:
            # Use database configuration
            self._cache = db_config
            self._cache_timestamp = time.time()
            return db_config

        # Fall back to environment variables
        env_config = self._get_env_fallback_config()
        self._cache = env_config
        self._cache_timestamp = time.time()
        return env_config

    def invalidate_cache(self):
        """Manually invalidate the cache"""
        self._cache = None
        self._cache_timestamp = 0


# Global instance
_llm_config_cache = LLMConfigCache()


async def get_llm_config() -> LLMConfig:
    """Get current LLM configuration (with caching)"""
    return await _llm_config_cache.get_config()


def invalidate_llm_config_cache():
    """Invalidate the LLM configuration cache"""
    _llm_config_cache.invalidate_cache()


@dataclass
class EmbeddingConfig:
    """Embedding configuration data class"""

    provider: str  # "jina", "bedrock", "openai", "local"
    # Jina fields
    jina_api_key: Optional[str]
    jina_model: Optional[str]
    jina_api_url: Optional[str]
    # Bedrock fields
    bedrock_model_id: Optional[str]
    # OpenAI fields
    openai_api_key: Optional[str]
    openai_model: Optional[str]
    openai_dimensions: Optional[int]
    # Local fields (vLLM-based)
    local_base_url: Optional[str]
    local_model: Optional[str]


class EmbeddingConfigCache:
    """
    Cached embedding configuration reader with PostgreSQL backend.
    Cache has a TTL of 90 seconds (1.5 minutes) to balance freshness with performance.
    """

    CACHE_TTL_SECONDS = 90  # 1.5 minutes

    def __init__(self):
        self._cache: Optional[EmbeddingConfig] = None
        self._cache_timestamp: float = 0

    def _is_cache_valid(self) -> bool:
        """Check if cache is still valid"""
        if self._cache is None:
            return False
        elapsed = time.time() - self._cache_timestamp
        return elapsed < self.CACHE_TTL_SECONDS

    async def _fetch_from_database(self) -> Optional[EmbeddingConfig]:
        """Fetch embedding configuration from database"""
        config_data = await fetch_embedding_config()

        if config_data:
            return EmbeddingConfig(
                provider=config_data.get("provider"),
                jina_api_key=config_data.get("jinaApiKey"),
                jina_model=config_data.get("jinaModel"),
                jina_api_url=config_data.get("jinaApiUrl"),
                bedrock_model_id=config_data.get("bedrockModelId"),
                openai_api_key=config_data.get("openaiApiKey"),
                openai_model=config_data.get("openaiModel"),
                openai_dimensions=config_data.get("openaiDimensions"),
                local_base_url=config_data.get("localBaseUrl"),
                local_model=config_data.get("localModel"),
            )
        return None

    def _get_env_fallback_config(self) -> EmbeddingConfig:
        """Get configuration from environment variables as fallback"""
        from config import (
            OPENAI_EMBEDDING_API_KEY,
            OPENAI_EMBEDDING_MODEL,
            OPENAI_EMBEDDING_DIMENSIONS,
            LOCAL_EMBEDDINGS_URL,
            LOCAL_EMBEDDINGS_MODEL,
        )

        return EmbeddingConfig(
            provider=EMBEDDING_PROVIDER,
            jina_api_key=JINA_API_KEY if JINA_API_KEY else None,
            jina_model=JINA_MODEL if JINA_MODEL else None,
            jina_api_url=JINA_API_URL if JINA_API_URL else None,
            bedrock_model_id=BEDROCK_EMBEDDING_MODEL_ID if BEDROCK_EMBEDDING_MODEL_ID else None,
            openai_api_key=OPENAI_EMBEDDING_API_KEY if OPENAI_EMBEDDING_API_KEY else None,
            openai_model=OPENAI_EMBEDDING_MODEL if OPENAI_EMBEDDING_MODEL else None,
            openai_dimensions=OPENAI_EMBEDDING_DIMENSIONS if OPENAI_EMBEDDING_DIMENSIONS else None,
            local_base_url=LOCAL_EMBEDDINGS_URL if LOCAL_EMBEDDINGS_URL else None,
            local_model=LOCAL_EMBEDDINGS_MODEL if LOCAL_EMBEDDINGS_MODEL else None,
        )

    async def get_config(self) -> EmbeddingConfig:
        """
        Get embedding configuration with caching.
        Priority: Database config -> Environment variables
        """
        # Return cached config if still valid
        if self._is_cache_valid():
            return self._cache  # type: ignore

        # Try to fetch from database
        db_config = await self._fetch_from_database()

        if db_config is not None:
            # Use database configuration
            self._cache = db_config
            self._cache_timestamp = time.time()
            return db_config

        # Fall back to environment variables
        env_config = self._get_env_fallback_config()
        self._cache = env_config
        self._cache_timestamp = time.time()
        return env_config

    def invalidate_cache(self):
        """Manually invalidate the cache"""
        self._cache = None
        self._cache_timestamp = 0


# Global instance
_embedding_config_cache = EmbeddingConfigCache()


async def get_embedding_config() -> EmbeddingConfig:
    """Get current embedding configuration (with caching)"""
    return await _embedding_config_cache.get_config()


def invalidate_embedding_config_cache():
    """Invalidate the embedding configuration cache"""
    _embedding_config_cache.invalidate_cache()

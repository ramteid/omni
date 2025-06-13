from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from typing import List, Optional
import logging
import asyncio
from concurrent.futures import ThreadPoolExecutor

from embeddings import (
    load_model,
    generate_embeddings_sync,
    TASK,
    MODEL_NAME,
)

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

app = FastAPI(title="Clio AI Service", version="0.1.0")

# Thread pool for async operations
_executor = ThreadPoolExecutor(max_workers=2)


# Pydantic models
class EmbeddingRequest(BaseModel):
    texts: List[str]
    task: Optional[str] = TASK  # Allow different tasks
    chunk_size: Optional[int] = 512  # Chunk size for fixed-size chunking
    chunking_mode: Optional[str] = "sentence"  # "sentence" or "fixed"


class EmbeddingResponse(BaseModel):
    embeddings: List[List[List[float]]]
    chunks_count: List[int]  # Number of chunks per text


class RAGRequest(BaseModel):
    query: str
    documents: List[str]


class RAGResponse(BaseModel):
    answer: str
    relevant_chunks: List[str]


@app.on_event("startup")
async def startup_event():
    """Load model on startup"""
    await asyncio.get_event_loop().run_in_executor(_executor, load_model)


@app.get("/health")
async def health_check():
    """Health check endpoint"""
    return {"status": "healthy", "service": "ai", "model": MODEL_NAME}


@app.post("/embeddings", response_model=EmbeddingResponse)
async def generate_embeddings(request: EmbeddingRequest):
    """Generate embeddings for input texts using configurable chunking"""
    logger.info(
        f"Generating embeddings for {len(request.texts)} texts with chunking_mode={request.chunking_mode}, chunk_size={request.chunk_size}"
    )

    # Validate chunking method
    valid_chunking_modes = ["sentence", "fixed"]
    if request.chunking_mode not in valid_chunking_modes:
        raise HTTPException(
            status_code=422,
            detail=f"Invalid chunking_mode: {request.chunking_mode}. Must be one of: {valid_chunking_modes}"
        )

    try:
        # Run embedding generation in thread pool to avoid blocking
        embeddings, chunks_count = await asyncio.get_event_loop().run_in_executor(
            _executor,
            generate_embeddings_sync,
            request.texts,
            request.task,
            request.chunk_size,
            request.chunking_mode,
        )

        logger.info(f"Generated embeddings with chunks: {chunks_count}")
        return EmbeddingResponse(embeddings=embeddings, chunks_count=chunks_count)

    except Exception as e:
        logger.error(f"Failed to generate embeddings: {str(e)}")
        raise HTTPException(status_code=500, detail=str(e))


@app.post("/rag", response_model=RAGResponse)
async def rag_inference(request: RAGRequest):
    """Perform RAG inference with retrieved documents"""
    # TODO: Implement RAG pipeline with vLLM integration
    logger.info(f"RAG inference for query: {request.query[:50]}...")

    # Placeholder response
    return RAGResponse(
        answer="This is a placeholder RAG response",
        relevant_chunks=request.documents[:3],  # Return top 3 chunks
    )


if __name__ == "__main__":
    import uvicorn

    uvicorn.run(app, host="0.0.0.0", port=8000)

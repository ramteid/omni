"""Prompt and PDF extraction endpoints."""

import asyncio
import logging
import multiprocessing
from concurrent.futures import ThreadPoolExecutor

from fastapi import APIRouter, HTTPException, Request
from fastapi.responses import StreamingResponse

from config import LLM_PROVIDER
from schemas import PromptRequest, PromptResponse
from processing import (
    PDFExtractionRequest,
    PDFExtractionResponse,
    extract_text_from_pdf,
)

from providers import LLMProvider

logger = logging.getLogger(__name__)
router = APIRouter(tags=["prompts"])


def _get_default_llm_provider(request: Request) -> LLMProvider | None:
    """Return the default LLM provider from app.state.models."""
    models = getattr(request.app.state, "models", None)
    if not models:
        return None
    default_id = getattr(request.app.state, "default_model_id", None)
    if default_id and default_id in models:
        return models[default_id]
    return next(iter(models.values()), None)


# Thread pool for async operations - scale based on CPU cores
# Reserve some cores for the web server and other processes
max_workers = max(2, min(multiprocessing.cpu_count() - 1, 8))
_executor = ThreadPoolExecutor(max_workers=max_workers)


@router.post("/prompt")
async def generate_response(request: Request, body: PromptRequest):
    """Generate a response from the configured LLM provider with streaming support."""
    llm_provider = _get_default_llm_provider(request)
    if not llm_provider:
        raise HTTPException(status_code=500, detail="LLM provider not initialized")

    logger.info(
        f"Generating response for prompt: {body.prompt[:50]}... (stream={body.stream}, provider={LLM_PROVIDER})"
    )

    if not body.stream:
        # Non-streaming response (keep for backward compatibility)
        return await _generate_non_streaming_response(request, body)

    # Streaming response
    async def stream_generator():
        try:
            async for event in llm_provider.stream_response(
                body.prompt,
                max_tokens=body.max_tokens,
                temperature=body.temperature,
                top_p=body.top_p,
            ):
                # Extract text content from MessageStreamEvent
                if event.type == "content_block_delta":
                    if event.delta.text:
                        yield event.delta.text
        except Exception as e:
            logger.error(f"Failed to generate streaming response: {str(e)}")
            yield f"Error: {str(e)}"

    return StreamingResponse(
        stream_generator(),
        media_type="text/plain",
        headers={"Cache-Control": "no-cache", "Connection": "keep-alive"},
    )


async def _generate_non_streaming_response(
    request: Request, body: PromptRequest
) -> PromptResponse:
    """Generate non-streaming response for backward compatibility."""
    llm_provider = _get_default_llm_provider(request)
    if not llm_provider:
        raise HTTPException(status_code=500, detail="LLM provider not initialized")

    try:
        generated_text = await llm_provider.generate_response(
            body.prompt,
            max_tokens=body.max_tokens,
            temperature=body.temperature,
            top_p=body.top_p,
        )

        logger.info(f"Successfully generated response of length: {len(generated_text)}")
        return PromptResponse(response=generated_text)

    except Exception as e:
        logger.error(f"Failed to generate response: {str(e)}")
        raise HTTPException(
            status_code=500, detail=f"Failed to generate response: {str(e)}"
        )


@router.post("/extract_pdf", response_model=PDFExtractionResponse)
async def extract_pdf(body: PDFExtractionRequest):
    """Extract text from a PDF file."""
    logger.info(f"Extracting text from PDF ({len(body.pdf_bytes)} bytes)")

    # Run PDF extraction in executor to avoid blocking
    loop = asyncio.get_event_loop()
    result = await loop.run_in_executor(
        _executor, extract_text_from_pdf, body.pdf_bytes
    )

    if result.error:
        logger.warning(f"PDF extraction completed with error: {result.error}")
    else:
        logger.info(
            f"PDF extraction successful: {result.page_count} pages, {len(result.text)} characters"
        )

    return result

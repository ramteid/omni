import logging
import os
import torch
import torch.nn.functional as F
from transformers import AutoModel, AutoTokenizer
import numpy as np
from typing import List, Tuple
import threading
from concurrent.futures import ThreadPoolExecutor
import code

logger = logging.getLogger(__name__)

# Global variables for model and tokenizer
_model = None
_tokenizer = None
_model_lock = threading.Lock()

# Model configuration
TASK = "retrieval.passage"
MODEL_NAME = os.getenv("EMBEDDING_MODEL", "jinaai/jina-embeddings-v3")
MAX_LENGTH = 8192  # Jina v3 supports up to 8K tokens


def load_model():
    """Load the Jina embeddings model and tokenizer"""
    global _model, _tokenizer

    with _model_lock:
        if _model is None:
            logger.info(f"Loading model {MODEL_NAME}...")
            _model = AutoModel.from_pretrained(MODEL_NAME, trust_remote_code=True)
            _tokenizer = AutoTokenizer.from_pretrained(
                MODEL_NAME, trust_remote_code=True
            )

            # Move to GPU if available
            if torch.cuda.is_available():
                _model = _model.cuda()
                logger.info("Model loaded on GPU")
            else:
                logger.info("Model loaded on CPU")

    return _model, _tokenizer


def chunk_by_sentences(input_text: str, tokenizer: callable) -> Tuple[List[str], List[Tuple[int, int]]]:
    """
    Split the input text into sentences using the tokenizer
    :param input_text: The text snippet to split into sentences
    :param tokenizer: The tokenizer to use
    :return: A tuple containing the list of text chunks and their corresponding token spans
    """
    inputs = tokenizer(input_text, return_tensors="pt", return_offsets_mapping=True)

    # Get token IDs for various sentence-ending punctuation marks
    sentence_terminators = {
        tokenizer.convert_tokens_to_ids("."),
        tokenizer.convert_tokens_to_ids("?"),
        tokenizer.convert_tokens_to_ids("!"),
    }
    # Filter out any None values (in case some punctuation isn't in vocabulary)
    sentence_terminators = {tid for tid in sentence_terminators if tid is not None}

    sep_id = tokenizer.convert_tokens_to_ids("[SEP]")
    eos_id = tokenizer.eos_token_id
    token_offsets = inputs["offset_mapping"][0]
    token_ids = inputs["input_ids"][0]

    chunk_positions = [
        (i, int(start + 1))
        for i, (token_id, (start, end)) in enumerate(zip(token_ids, token_offsets))
        if token_id.item() in sentence_terminators
        and i + 1 < len(token_ids)
        and (
            token_offsets[i + 1][0] - token_offsets[i][1] > 0
            or token_ids[i + 1] == sep_id
            or token_ids[i + 1] == eos_id
        )
    ]
    chunks = [
        input_text[x[1] : y[1]]
        for x, y in zip([(1, 0)] + chunk_positions[:-1], chunk_positions)
    ]
    span_annotations = [
        (x[0], y[0]) for (x, y) in zip([(1, 0)] + chunk_positions[:-1], chunk_positions)
    ]
    return chunks, span_annotations


def apply_fixed_size_chunking(
    token_embeddings: torch.Tensor, input_ids: torch.Tensor, chunk_size: int
) -> List[torch.Tensor]:
    """Apply late chunking to token embeddings using fixed chunk size"""
    chunks = []
    seq_len = token_embeddings.shape[1]

    for i in range(0, seq_len, chunk_size):
        end_idx = min(i + chunk_size, seq_len)
        chunk_embeddings = token_embeddings[:, i:end_idx, :]

        # Mean pooling for chunk representation
        chunk_embedding = torch.mean(chunk_embeddings, dim=1)
        # Normalize to unit norm
        chunk_embedding = F.normalize(chunk_embedding, p=2, dim=-1)
        chunks.append(chunk_embedding)

    return chunks


def apply_sentence_chunking(
    token_embeddings: torch.Tensor, span_annotations: List[tuple]
) -> List[torch.Tensor]:
    """Apply sentence-based chunking using span annotations from chunk_by_sentences"""
    chunks = []

    for start_idx, end_idx in span_annotations:
        # Extract embeddings for this sentence span
        # Ensure indices are within bounds
        start_idx = max(0, start_idx)
        end_idx = min(token_embeddings.shape[1], end_idx)

        if start_idx < end_idx:  # Valid span
            # Selecting everything from the batch dimension works here because we are processing one batch entry
            # at a time. If we process a batch of inputs this won't work because span annotations will be different
            # for each entry in the batch.
            sentence_embeddings = token_embeddings[:, start_idx:end_idx, :]

            # Mean pooling for sentence representation
            sentence_embedding = torch.mean(sentence_embeddings, dim=1)
            # Normalize to unit norm
            sentence_embedding = F.normalize(sentence_embedding, p=2, dim=-1)
            chunks.append(sentence_embedding)

    return chunks


def generate_embeddings_sync(
    texts: List[str], task: str, chunk_size: int, chunking_mode: str
) -> Tuple[List[List[float]], List[int]]:
    """Synchronous embedding generation with configurable chunking"""
    try:
        logger.info(f"Starting embedding generation for {len(texts)} texts")
        model, tokenizer = load_model()
        logger.info("Model and tokenizer loaded successfully")

        # Batch tokenization
        logger.info("Tokenizing all texts in batch...")
        inputs = tokenizer(
            texts, return_tensors="pt", truncation=True, max_length=MAX_LENGTH, padding=True
        )
        logger.info(f"Batch tokenization complete, input_ids shape: {inputs['input_ids'].shape}")

        if torch.cuda.is_available():
            logger.info("Moving batch inputs to GPU...")
            inputs = {k: v.cuda() for k, v in inputs.items()}
            logger.info("Batch inputs moved to GPU")

        # Get task ID for adapter
        logger.info(f"Getting task ID for task: {task}")
        task_id = model._adaptation_map.get(task, model._adaptation_map[TASK])
        num_examples = inputs["input_ids"].shape[0]
        logger.info(f"Task ID: {task_id}, num_examples: {num_examples}")

        device = model.device
        logger.info(f"Model device: {device}")
        adapter_mask = torch.full(
            (num_examples,), task_id, dtype=torch.int32, device=device
        )
        logger.info(f"Created adapter_mask: shape={adapter_mask.shape}, dtype={adapter_mask.dtype}, device={adapter_mask.device}")

        # Single forward pass for all texts
        logger.info("Starting batch model forward pass...")
        with torch.no_grad():
            model_output = model(
                **inputs, adapter_mask=adapter_mask, return_dict=True
            )
            logger.info("Batch model forward pass completed")
            batch_token_embeddings = model_output.last_hidden_state
            logger.info(f"Batch token embeddings shape: {batch_token_embeddings.shape}, dtype: {batch_token_embeddings.dtype}")

        # Process each text's embeddings from the batch
        all_embeddings = []
        chunks_count = []

        for i, text in enumerate(texts):
            logger.info(f"Processing embeddings for text {i+1}/{len(texts)}")
            
            # Extract embeddings for this specific text from the batch
            token_embeddings = batch_token_embeddings[i:i+1]  # Keep batch dimension
            text_inputs = {k: v[i:i+1] for k, v in inputs.items()}  # Extract inputs for this text
            
            # Apply chunking based on the selected mode
            logger.info(f"Applying chunking mode: {chunking_mode}")
            if chunking_mode == "sentence":
                # Use sentence-based chunking
                logger.info("Processing sentence-based chunking...")
                _, span_annotations = chunk_by_sentences(text, tokenizer)
                logger.info(f"Found {len(span_annotations)} sentence spans")
                chunk_embeddings = apply_sentence_chunking(
                    token_embeddings, span_annotations
                )
                logger.info(f"Sentence chunking produced {len(chunk_embeddings)} chunks")
            else:
                # Use fixed-size chunking (default)
                logger.info("Processing fixed-size chunking...")
                chunk_embeddings = apply_fixed_size_chunking(
                    token_embeddings, text_inputs["input_ids"], chunk_size
                )
                logger.info(f"Fixed chunking produced {len(chunk_embeddings)} chunks")

            # Convert to numpy and store
            logger.info("Converting embeddings to numpy...")
            text_embeddings = []
            for j, chunk_emb in enumerate(chunk_embeddings):
                logger.info(f"Converting chunk {j+1}/{len(chunk_embeddings)} to numpy")
                # Convert BFloat16 to Float32 before numpy conversion
                chunk_emb_np = chunk_emb.float().cpu().numpy().tolist()
                text_embeddings.append(chunk_emb_np[0])

            # Flatten the embeddings for this text
            logger.info(f"Extending all_embeddings with text_embeddings of len {len(text_embeddings)}")
            logger.info(f"Text embeddings array content lens: {[len(x) for x in text_embeddings]}")
            all_embeddings.append(text_embeddings)
            chunks_count.append(len(chunk_embeddings))
            logger.info(f"Completed processing text {i+1}/{len(texts)}")

        logger.info("All embeddings generated successfully")
        return all_embeddings, chunks_count

    except Exception as e:
        logger.error(f"Error generating embeddings: {str(e)}")
        import traceback
        logger.error(f"Full traceback: {traceback.format_exc()}")
        raise Exception(f"Embedding generation failed: {str(e)}")
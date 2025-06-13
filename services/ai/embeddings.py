import logging
import torch
from transformers import AutoModel, AutoTokenizer
import numpy as np
from typing import List, Tuple
import threading
from concurrent.futures import ThreadPoolExecutor

logger = logging.getLogger(__name__)

# Global variables for model and tokenizer
_model = None
_tokenizer = None
_model_lock = threading.Lock()

# Model configuration
TASK = "retrieval.passage"
MODEL_NAME = "jinaai/jina-embeddings-v3"
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


def apply_late_chunking(
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
            sentence_embeddings = token_embeddings[:, start_idx:end_idx, :]

            # Mean pooling for sentence representation
            sentence_embedding = torch.mean(sentence_embeddings, dim=1)
            chunks.append(sentence_embedding)

    return chunks


def generate_embeddings_sync(
    texts: List[str], task: str, chunk_size: int, chunking_mode: str
) -> Tuple[List[List[float]], List[int]]:
    """Synchronous embedding generation with configurable chunking"""
    try:
        model, tokenizer = load_model()

        all_embeddings = []
        chunks_count = []

        for text in texts:
            # Tokenize the text
            inputs = tokenizer(
                text, return_tensors="pt", truncation=True, max_length=MAX_LENGTH
            )

            if torch.cuda.is_available():
                inputs = {k: v.cuda() for k, v in inputs.items()}

            # Get task ID for adapter
            task_id = model._adaptation_map.get(task, model._adaptation_map[TASK])
            num_examples = inputs["input_ids"].shape[0]

            device = model.device
            adapter_mask = torch.full(
                (num_examples,), task_id, dtype=torch.int32, device=device
            )

            # Forward pass to get token embeddings
            with torch.no_grad():
                model_output = model(
                    **inputs, adapter_mask=adapter_mask, return_dict=True
                )
                token_embeddings = model_output.last_hidden_state

            # Apply chunking based on the selected mode
            if chunking_mode == "sentence":
                # Use sentence-based chunking
                _, span_annotations = chunk_by_sentences(text, tokenizer)
                chunk_embeddings = apply_sentence_chunking(
                    token_embeddings, span_annotations
                )
            else:
                # Use fixed-size chunking (default)
                chunk_embeddings = apply_late_chunking(
                    token_embeddings, inputs["input_ids"], chunk_size
                )

            # Convert to numpy and store
            text_embeddings = []
            for chunk_emb in chunk_embeddings:
                chunk_emb_np = chunk_emb.cpu().numpy().tolist()
                text_embeddings.extend(chunk_emb_np)

            all_embeddings.append(
                text_embeddings[0] if len(text_embeddings) == 1 else text_embeddings
            )
            chunks_count.append(len(chunk_embeddings))

        return all_embeddings, chunks_count

    except Exception as e:
        logger.error(f"Error generating embeddings: {str(e)}")
        raise Exception(f"Embedding generation failed: {str(e)}")
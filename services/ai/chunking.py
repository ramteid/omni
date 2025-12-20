import bisect
import logging
import re
from typing import Dict, List, Optional, Tuple, Union

from llama_index.core.node_parser import SemanticSplitterNodeParser
from llama_index.core.schema import Document
from llama_index.embeddings.huggingface import HuggingFaceEmbedding
from transformers import AutoTokenizer

# Set the logging level to WARNING to suppress INFO and DEBUG messages
logging.getLogger("sentence_transformers").setLevel(logging.WARNING)

CHUNKING_STRATEGIES = ["semantic", "fixed", "sentence"]


def chunk_by_sentences_chars(text: str, max_chars: int) -> list[tuple[int, int]]:
    """Chunk text by sentences, keeping chunks under max_chars (character-based)."""
    sentence_pattern = r"[.!?]+[\s]+"
    sentences = []
    last_end = 0

    for match in re.finditer(sentence_pattern, text):
        sentence_end = match.end()
        if last_end < sentence_end:
            sentences.append((last_end, sentence_end))
        last_end = sentence_end

    if last_end < len(text):
        sentences.append((last_end, len(text)))

    if not sentences:
        return [(0, len(text))]

    chunks = []
    chunk_start = 0
    last_sentence_end = 0

    for sent_start, sent_end in sentences:
        current_chunk_len = sent_end - chunk_start

        if current_chunk_len > max_chars and last_sentence_end > chunk_start:
            chunks.append((chunk_start, last_sentence_end))
            chunk_start = last_sentence_end

        last_sentence_end = sent_end

    if chunk_start < len(text):
        chunks.append((chunk_start, len(text)))

    return chunks if chunks else [(0, len(text))]


def chunk_by_chars(text: str, max_chars: int) -> list[tuple[int, int]]:
    """Chunk text by fixed character count."""
    if not text or max_chars < 1:
        return []

    chunks = []
    for i in range(0, len(text), max_chars):
        chunk_end = min(i + max_chars, len(text))
        chunks.append((i, chunk_end))

    return chunks if chunks else [(0, len(text))]


class Chunker:
    def __init__(
        self,
        chunking_strategy: str,
    ):
        if chunking_strategy not in CHUNKING_STRATEGIES:
            raise ValueError("Unsupported chunking strategy: ", chunking_strategy)
        self.chunking_strategy = chunking_strategy
        self.embed_model = None
        self.embedding_model_name = None

    def _setup_semantic_chunking(self, embedding_model_name):
        if embedding_model_name:
            self.embedding_model_name = embedding_model_name

        self.embed_model = HuggingFaceEmbedding(
            model_name=self.embedding_model_name,
            trust_remote_code=True,
            embed_batch_size=1,
        )
        self.splitter = SemanticSplitterNodeParser(
            embed_model=self.embed_model,
            show_progress=False,
        )

    def chunk_semantically(
        self,
        text: str,
        tokenizer: "AutoTokenizer",
        embedding_model_name: Optional[str] = None,
    ) -> Tuple[List[Tuple[int, int]], List[Tuple[int, int]]]:
        if self.embed_model is None:
            self._setup_semantic_chunking(embedding_model_name)

        # Get semantic nodes
        nodes = [
            (node.start_char_idx, node.end_char_idx)
            for node in self.splitter.get_nodes_from_documents(
                [Document(text=text)], show_progress=False
            )
        ]

        # Tokenize the entire text
        tokens = tokenizer.encode_plus(
            text,
            return_offsets_mapping=True,
            add_special_tokens=False,
            padding=True,
            truncation=True,
        )
        token_offsets = tokens.offset_mapping

        token_spans = []
        char_spans = []

        for char_start, char_end in nodes:
            # Validate character bounds
            if char_start >= char_end or char_start < 0 or char_end > len(text):
                continue

            # Convert char indices to token indices
            start_chunk_index = bisect.bisect_left(
                [offset[0] for offset in token_offsets], char_start
            )
            end_chunk_index = bisect.bisect_right(
                [offset[1] for offset in token_offsets], char_end
            )

            # Validate token bounds and ensure start < end
            if (
                start_chunk_index < len(token_offsets)
                and end_chunk_index <= len(token_offsets)
                and start_chunk_index < end_chunk_index
            ):
                token_spans.append((start_chunk_index, end_chunk_index))
                char_spans.append((char_start, char_end))

        return token_spans, char_spans

    def chunk_by_tokens(
        self,
        text: str,
        chunk_size: int,
        tokenizer: "AutoTokenizer",
    ) -> Tuple[List[Tuple[int, int]], List[Tuple[int, int]]]:
        if not text or chunk_size < 1:
            return [], []

        tokens = tokenizer.encode_plus(
            text, return_offsets_mapping=True, add_special_tokens=False
        )
        token_offsets = tokens.offset_mapping

        token_spans = []
        char_spans = []
        for i in range(0, len(token_offsets), chunk_size):
            chunk_end = min(i + chunk_size, len(token_offsets))
            if chunk_end > i:  # Ensure valid span
                # Get character indices from token offsets
                char_start = token_offsets[i][0]
                char_end = token_offsets[chunk_end - 1][1]

                # Validate character bounds
                if char_start < char_end and char_start >= 0 and char_end <= len(text):
                    token_spans.append((i, chunk_end))
                    char_spans.append((char_start, char_end))

        return token_spans, char_spans

    def chunk_by_sentences(
        self,
        text: str,
        chunk_size: int,
        tokenizer: "AutoTokenizer",
    ) -> Tuple[List[Tuple[int, int]], List[Tuple[int, int]]]:
        """Chunk text by sentences, keeping chunks under chunk_size tokens"""
        if not text or chunk_size < 1:
            return [], []

        tokens = tokenizer.encode_plus(
            text, return_offsets_mapping=True, add_special_tokens=False
        )
        token_offsets = tokens.offset_mapping

        if not token_offsets:
            return [], []

        token_spans = []
        char_spans = []
        chunk_start = 0
        last_sentence_end = 0

        for i in range(len(token_offsets)):
            # Check if this is a sentence boundary
            if (
                i < len(tokens.tokens(0))
                and tokens.tokens(0)[i] in (".", "!", "?")
                and (
                    (len(tokens.tokens(0)) == i + 1)
                    or (
                        i + 1 < len(token_offsets)
                        and tokens.token_to_chars(i).end
                        != tokens.token_to_chars(i + 1).start
                    )
                )
            ):
                # This is a sentence boundary
                sentence_end = i + 1
                current_chunk_tokens = sentence_end - chunk_start

                # Check if adding this sentence would exceed the limit
                if (
                    current_chunk_tokens > chunk_size
                    and last_sentence_end > chunk_start
                ):
                    # Create chunk up to the previous sentence
                    char_start = token_offsets[chunk_start][0]
                    char_end = token_offsets[last_sentence_end - 1][1]

                    if (
                        char_start < char_end
                        and char_start >= 0
                        and char_end <= len(text)
                    ):
                        token_spans.append((chunk_start, last_sentence_end))
                        char_spans.append((char_start, char_end))

                    # Start new chunk from the current sentence
                    chunk_start = last_sentence_end

                # Update last sentence end
                last_sentence_end = sentence_end

        # Handle the last chunk
        if chunk_start < len(token_offsets):
            char_start = token_offsets[chunk_start][0]
            char_end = token_offsets[-1][1]

            if char_start < char_end and char_start >= 0 and char_end <= len(text):
                token_spans.append((chunk_start, len(token_offsets)))
                char_spans.append((char_start, char_end))

        return token_spans, char_spans

    def chunk(
        self,
        text: str,
        tokenizer: "AutoTokenizer",
        chunking_strategy: str = None,
        chunk_size: Optional[int] = None,
        embedding_model_name: Optional[str] = None,
    ):
        chunking_strategy = chunking_strategy or self.chunking_strategy
        if chunking_strategy == "semantic":
            return self.chunk_semantically(
                text,
                embedding_model_name=embedding_model_name,
                tokenizer=tokenizer,
            )
        elif chunking_strategy == "fixed":
            if chunk_size < 4:
                raise ValueError("Chunk size must be >= 4.")
            return self.chunk_by_tokens(text, chunk_size, tokenizer)
        elif chunking_strategy == "sentence":
            return self.chunk_by_sentences(text, chunk_size, tokenizer)
        else:
            raise ValueError("Unsupported chunking strategy")

#!/usr/bin/env python3
"""
Comprehensive test suite for embeddings_v2.py

Tests chunking algorithms, embedding generation, and semantic correctness.
"""

import pytest
import torch
import numpy as np
from typing import List, Tuple, Dict
import time
import math

# Import the module under test
from embeddings_v2 import (
    load_model,
    tokenize,
    forward,
    chunk_by_sentences,
    generate_sentence_chunks,
    apply_late_chunking,
    generate_embeddings_sync,
    Chunk,
    MAX_MODEL_SEQ_LEN,
    QUERY_TASK,
    PASSAGE_TASK,
)


# Test fixtures
@pytest.fixture(scope="session")
def model_and_tokenizer():
    """Load model and tokenizer once for all tests."""
    return load_model()


@pytest.fixture
def sample_texts():
    """Sample texts for testing."""
    return [
        "This is a simple sentence.",
        "First sentence. Second sentence! Third sentence?",
        "A longer paragraph with multiple sentences. It contains various punctuation marks! Does it handle questions? Yes, it does. And it has many different sentence structures.",
        "Short.",
        "",  # Empty string
        "NoperiodattheendofSentence",
        "Multiple...ellipses...and weird!!! punctuation??? patterns.",
    ]


@pytest.fixture
def diverse_document():
    """Multi-paragraph document with diverse topics for semantic testing."""
    return """Cooking pasta requires attention to timing and technique. Start by bringing a large pot of salted water to a rolling boil. Add the pasta and stir occasionally to prevent sticking. Fresh pasta typically cooks in 2-3 minutes, while dried pasta takes 8-12 minutes depending on the shape. Test for doneness by tasting - the pasta should be al dente, firm to the bite. Drain immediately and reserve some pasta water for adjusting sauce consistency.

The solar system consists of eight planets orbiting the Sun, along with countless asteroids, comets, and moons. Mercury is the closest planet to the Sun, while Neptune is the farthest. Earth is the third planet from the Sun and the only known planet to harbor life. Jupiter is the largest planet, with a mass greater than all other planets combined. The asteroid belt lies between Mars and Jupiter, containing thousands of rocky objects.

Gardening in spring requires careful planning and timing. Begin by preparing your soil with compost and organic matter. Cool-season crops like lettuce, spinach, and peas can be planted early, while warm-season vegetables like tomatoes and peppers should wait until after the last frost. Mulching helps retain moisture and suppress weeds. Regular watering is essential, especially for newly planted seeds and transplants.

The history of jazz music begins in New Orleans in the late 19th century. Jazz emerged from a blend of African American musical traditions, including blues, ragtime, and spirituals. Key pioneers include Buddy Bolden, Jelly Roll Morton, and Louis Armstrong. The genre evolved through various styles including swing, bebop, and fusion. Jazz emphasizes improvisation, syncopated rhythms, and blue notes. The music spread from New Orleans to Chicago, New York, and eventually worldwide."""


@pytest.fixture
def semantic_queries():
    """Queries targeting specific topics in the diverse document."""
    return [
        ("How long should I cook spaghetti?", "cooking"),
        ("Which planet is closest to the Sun?", "solar_system"),
        ("When should I plant tomatoes?", "gardening"),
        ("Who were the pioneers of jazz music?", "jazz_music"),
        ("What is al dente pasta?", "cooking"),
        ("How many planets are in our solar system?", "solar_system"),
        ("What vegetables can I plant in early spring?", "gardening"),
        ("Where did jazz music originate?", "jazz_music"),
    ]


class TestCoreInfrastructure:
    """Test core functionality: model loading, tokenization, forward pass."""

    def test_model_loading(self, model_and_tokenizer):
        """Test that model and tokenizer are loaded correctly."""
        model, tokenizer = model_and_tokenizer
        assert model is not None
        assert tokenizer is not None
        assert hasattr(model, "_adaptation_map")
        assert QUERY_TASK in model._adaptation_map
        assert PASSAGE_TASK in model._adaptation_map

    def test_model_caching(self):
        """Test that multiple calls to load_model return the same instances."""
        model1, tokenizer1 = load_model()
        model2, tokenizer2 = load_model()
        assert model1 is model2
        assert tokenizer1 is tokenizer2

    def test_tokenization_basic(self, model_and_tokenizer, sample_texts):
        """Test basic tokenization functionality."""
        _, tokenizer = model_and_tokenizer
        inputs = tokenize(tokenizer, sample_texts)

        assert "input_ids" in inputs
        assert "attention_mask" in inputs
        assert "offset_mapping" in inputs
        assert inputs.input_ids.shape[0] == len(sample_texts)
        assert inputs.input_ids.shape == inputs.attention_mask.shape

    def test_tokenization_empty_input(self, model_and_tokenizer):
        """Test tokenization with empty input."""
        _, tokenizer = model_and_tokenizer
        inputs = tokenize(tokenizer, [""])

        assert inputs.input_ids.shape[0] == 1
        assert inputs.input_ids.shape[1] > 0  # Should have at least special tokens

    def test_forward_pass_shapes(self, model_and_tokenizer, sample_texts):
        """Test forward pass produces correct output shapes."""
        model, tokenizer = model_and_tokenizer
        inputs = tokenize(tokenizer, sample_texts)

        # Test passage task
        embeddings = forward(model, inputs, task=PASSAGE_TASK)
        assert embeddings.shape[0] == len(sample_texts)
        assert embeddings.shape[1] == inputs.input_ids.shape[1]
        assert embeddings.shape[2] > 0  # Embedding dimension

        # Test query task
        embeddings_query = forward(model, inputs, task=QUERY_TASK)
        assert embeddings_query.shape == embeddings.shape

    def test_forward_pass_different_tasks(self, model_and_tokenizer):
        """Test that different tasks produce different embeddings."""
        model, tokenizer = model_and_tokenizer
        text = ["This is a test sentence."]
        inputs = tokenize(tokenizer, text)

        embeddings_passage = forward(model, inputs, task=PASSAGE_TASK)
        embeddings_query = forward(model, inputs, task=QUERY_TASK)

        # Embeddings should be different for different tasks
        assert not torch.allclose(embeddings_passage, embeddings_query, atol=1e-6)


class TestChunkingSentences:
    """Test chunk_by_sentences function."""

    def test_basic_chunking(self, model_and_tokenizer):
        """Test basic sentence chunking functionality."""
        _, tokenizer = model_and_tokenizer
        texts = ["First sentence. Second sentence. Third sentence. Fourth sentence."]
        inputs = tokenize(tokenizer, texts)

        char_spans, token_spans = chunk_by_sentences(inputs, tokenizer, chunk_size=50)

        assert len(char_spans) == 1
        assert len(token_spans) == 1
        assert (
            len(char_spans[0]) == 1
        )  # 4 sentences with chunk_size=50 produces 1 chunk
        assert len(token_spans[0]) == len(char_spans[0])

    def test_chunk_span_validation(self, model_and_tokenizer):
        """Test that chunk spans are valid and non-overlapping."""
        _, tokenizer = model_and_tokenizer
        texts = [
            "First sentence. Second sentence. Third sentence. Fourth sentence. Fifth sentence."
        ]
        inputs = tokenize(tokenizer, texts)

        char_spans, token_spans = chunk_by_sentences(inputs, tokenizer, chunk_size=30)

        # Validate character spans
        for spans in char_spans:
            for start, end in spans:
                assert start < end, f"Invalid char span: {start} >= {end}"
                assert start >= 0, f"Negative char start: {start}"

        # Validate token spans
        for spans in token_spans:
            for start, end in spans:
                assert start < end, f"Invalid token span: {start} >= {end}"
                assert start >= 0, f"Negative token start: {start}"

    def test_chunk_content_extraction(self, model_and_tokenizer):
        """Test that chunk spans correctly extract content."""
        _, tokenizer = model_and_tokenizer
        text = "First sentence. Second sentence."
        texts = [text]
        inputs = tokenize(tokenizer, texts)

        char_spans, _ = chunk_by_sentences(inputs, tokenizer, chunk_size=100)

        # Extract text using char spans
        for spans in char_spans:
            for start, end in spans:
                chunk_text = text[start:end]
                assert len(chunk_text) > 0
                assert chunk_text.strip()  # Should contain non-whitespace content

    def test_different_chunk_sizes(self, model_and_tokenizer):
        """Test chunking with different chunk sizes."""
        _, tokenizer = model_and_tokenizer
        texts = [
            "Sentence one. Sentence two. Sentence three. Sentence four. Sentence five."
        ]
        inputs = tokenize(tokenizer, texts)

        # Test with small chunk size (should create more chunks)
        char_spans_small, _ = chunk_by_sentences(inputs, tokenizer, chunk_size=20)

        # Test with large chunk size (should create fewer chunks)
        char_spans_large, _ = chunk_by_sentences(inputs, tokenizer, chunk_size=200)

        assert len(char_spans_small[0]) >= len(char_spans_large[0])

    def test_single_sentence(self, model_and_tokenizer):
        """Test chunking with single sentence."""
        _, tokenizer = model_and_tokenizer
        texts = ["This is a single sentence."]
        inputs = tokenize(tokenizer, texts)

        char_spans, token_spans = chunk_by_sentences(inputs, tokenizer, chunk_size=50)

        assert len(char_spans[0]) == 1
        assert len(token_spans[0]) == 1

    def test_no_punctuation(self, model_and_tokenizer):
        """Test chunking with text that has no sentence punctuation."""
        _, tokenizer = model_and_tokenizer
        texts = ["This text has no sentence ending punctuation"]
        inputs = tokenize(tokenizer, texts)

        char_spans, token_spans = chunk_by_sentences(inputs, tokenizer, chunk_size=50)

        # Text without punctuation still creates exactly one chunk
        assert len(char_spans[0]) == 1
        assert len(token_spans[0]) == 1


class TestChunkingKSentences:
    """Test generate_sentence_chunks function."""

    def test_k_sentence_chunking(self, model_and_tokenizer):
        """Test K consecutive sentence chunking."""
        _, tokenizer = model_and_tokenizer
        texts = ["First. Second. Third. Fourth. Fifth. Sixth."]
        inputs = tokenize(tokenizer, texts)

        char_spans, token_spans = generate_sentence_chunks(
            inputs, tokenizer, k_sentences=3
        )

        assert len(char_spans) == 1
        assert len(token_spans) == 1
        assert (
            len(char_spans[0]) == 2
        )  # 6 sentences with k_sentences=3 produces 2 chunks
        assert len(token_spans[0]) == len(char_spans[0])

    def test_overlapping_chunks(self, model_and_tokenizer):
        """Test that K-sentence chunks have proper overlapping."""
        _, tokenizer = model_and_tokenizer
        texts = ["First. Second. Third. Fourth. Fifth. Sixth. Seventh."]
        inputs = tokenize(tokenizer, texts)

        char_spans, _ = generate_sentence_chunks(inputs, tokenizer, k_sentences=3)

        # With k=3, we should get overlapping chunks
        # First chunk: sentences 1-3, Second chunk: sentences 4-6, etc.
        spans = char_spans[0]
        if len(spans) > 1:
            # Check that chunks don't overlap in this implementation
            # (since it's non-overlapping K-sentence chunks)
            for i in range(len(spans) - 1):
                assert spans[i][1] <= spans[i + 1][0]

    def test_different_k_values(self, model_and_tokenizer):
        """Test different K values for sentence chunking."""
        _, tokenizer = model_and_tokenizer
        texts = ["One. Two. Three. Four. Five. Six. Seven. Eight."]
        inputs = tokenize(tokenizer, texts)

        # Test with k=2
        char_spans_k2, _ = generate_sentence_chunks(inputs, tokenizer, k_sentences=2)

        # Test with k=5
        char_spans_k5, _ = generate_sentence_chunks(inputs, tokenizer, k_sentences=5)

        # 8 sentences: k=2 produces 4 chunks, k=5 produces 2 chunks
        assert len(char_spans_k2[0]) == 4
        assert len(char_spans_k5[0]) == 2
        assert len(char_spans_k2[0]) >= len(
            char_spans_k5[0]
        )  # Smaller K creates more chunks

    def test_k_larger_than_sentences(self, model_and_tokenizer):
        """Test K value larger than number of sentences."""
        _, tokenizer = model_and_tokenizer
        texts = ["Only one sentence here."]
        inputs = tokenize(tokenizer, texts)

        char_spans, token_spans = generate_sentence_chunks(
            inputs, tokenizer, k_sentences=5
        )

        # Should still create one chunk containing the single sentence
        assert len(char_spans[0]) == 1
        assert len(token_spans[0]) == 1


class TestLateChunking:
    """Test apply_late_chunking function."""

    def test_late_chunking_basic(self, model_and_tokenizer):
        """Test basic late chunking functionality."""
        model, tokenizer = model_and_tokenizer
        texts = ["First sentence. Second sentence."]
        inputs = tokenize(tokenizer, texts)

        # Get embeddings
        embeddings = forward(model, inputs, task=PASSAGE_TASK)

        # Get chunk spans
        _, token_spans = chunk_by_sentences(inputs, tokenizer, chunk_size=50)

        # Apply late chunking
        chunk_embeddings = apply_late_chunking(embeddings, token_spans)

        assert isinstance(chunk_embeddings, torch.Tensor)
        assert chunk_embeddings.shape[0] == sum(len(spans) for spans in token_spans)
        assert chunk_embeddings.shape[1] == embeddings.shape[2]

    def test_embedding_normalization(self, model_and_tokenizer):
        """Test that embeddings are properly normalized."""
        model, tokenizer = model_and_tokenizer
        texts = ["Test sentence for normalization."]
        inputs = tokenize(tokenizer, texts)

        embeddings = forward(model, inputs, task=PASSAGE_TASK)
        _, token_spans = chunk_by_sentences(inputs, tokenizer, chunk_size=100)

        chunk_embeddings = apply_late_chunking(embeddings, token_spans)

        # Check that embeddings are normalized (unit norm)
        norms = torch.norm(chunk_embeddings, dim=1)
        assert torch.allclose(norms, torch.ones_like(norms), atol=1e-6)

    def test_different_batch_sizes(self, model_and_tokenizer):
        """Test late chunking with different batch sizes."""
        model, tokenizer = model_and_tokenizer

        # Single text
        texts_single = ["Single sentence."]
        inputs_single = tokenize(tokenizer, texts_single)
        embeddings_single = forward(model, inputs_single, task=PASSAGE_TASK)
        _, token_spans_single = chunk_by_sentences(
            inputs_single, tokenizer, chunk_size=50
        )
        chunk_embeddings_single = apply_late_chunking(
            embeddings_single, token_spans_single
        )

        # Multiple texts
        texts_multi = ["First text.", "Second text.", "Third text."]
        inputs_multi = tokenize(tokenizer, texts_multi)
        embeddings_multi = forward(model, inputs_multi, task=PASSAGE_TASK)
        _, token_spans_multi = chunk_by_sentences(
            inputs_multi, tokenizer, chunk_size=50
        )
        chunk_embeddings_multi = apply_late_chunking(
            embeddings_multi, token_spans_multi
        )

        assert chunk_embeddings_single.shape[1] == chunk_embeddings_multi.shape[1]
        assert chunk_embeddings_multi.shape[0] >= chunk_embeddings_single.shape[0]


class TestIntegration:
    """Test generate_embeddings_sync integration function."""

    def test_no_chunking_mode(self, sample_texts):
        """Test embeddings generation without chunking."""
        result = generate_embeddings_sync(
            texts=sample_texts, task=PASSAGE_TASK, chunk_size=512, chunking_mode="none"
        )

        assert len(result) == len(sample_texts)
        for i, chunks in enumerate(result):
            assert len(chunks) == 1  # Should have exactly one chunk per text
            assert isinstance(chunks[0], Chunk)
            assert chunks[0].span == (0, len(sample_texts[i]))
            assert isinstance(chunks[0].embedding, list)
            assert len(chunks[0].embedding) > 0

    def test_chunking_mode(self, sample_texts):
        """Test embeddings generation with chunking."""
        result = generate_embeddings_sync(
            texts=sample_texts[:3],  # Use first 3 texts to avoid too much computation
            task=PASSAGE_TASK,
            chunk_size=50,
            chunking_mode="sentences",
        )

        assert len(result) == 3
        for chunks in result:
            assert len(chunks) > 0
            for chunk in chunks:
                assert isinstance(chunk, Chunk)
                assert isinstance(chunk.span, tuple)
                assert len(chunk.span) == 2
                assert chunk.span[0] < chunk.span[1]
                assert isinstance(chunk.embedding, list)
                assert len(chunk.embedding) > 0

    def test_query_vs_passage_task(self):
        """Test that query and passage tasks produce different embeddings."""
        texts = ["This is a test sentence."]

        result_passage = generate_embeddings_sync(
            texts=texts, task=PASSAGE_TASK, chunk_size=512, chunking_mode="none"
        )

        result_query = generate_embeddings_sync(
            texts=texts, task=QUERY_TASK, chunk_size=512, chunking_mode="none"
        )

        # Embeddings should be different for different tasks
        embedding_passage = result_passage[0][0].embedding
        embedding_query = result_query[0][0].embedding

        assert not np.allclose(embedding_passage, embedding_query, atol=1e-6)

    def test_different_chunk_sizes(self):
        """Test chunking with different chunk sizes."""
        texts = ["First sentence. Second sentence. Third sentence. Fourth sentence."]

        result_small = generate_embeddings_sync(
            texts=texts, task=PASSAGE_TASK, chunk_size=20, chunking_mode="sentences"
        )

        result_large = generate_embeddings_sync(
            texts=texts, task=PASSAGE_TASK, chunk_size=200, chunking_mode="sentences"
        )

        # Smaller chunk size should generally produce more chunks
        assert len(result_small[0]) >= len(result_large[0])


class TestSemanticCorrectness:
    """Test semantic correctness with real query-document matching."""

    def test_semantic_search_accuracy(self, diverse_document, semantic_queries):
        """Test that queries match the correct document sections using full chunking pipeline."""
        # Generate embeddings for the entire document using chunking
        doc_result = generate_embeddings_sync(
            texts=[diverse_document],
            task=PASSAGE_TASK,
            chunk_size=200,  # Smaller chunks for more granular matching
            chunking_mode="sentences",
        )

        # Extract all chunk embeddings and spans
        chunks = doc_result[0]
        chunk_embeddings = torch.tensor([chunk.embedding for chunk in chunks])
        chunk_spans = [chunk.span for chunk in chunks]

        # Document should be chunked into exactly 8 chunks
        print(f"DEBUG: Document chunked into {len(chunks)} chunks")
        assert len(chunks) == 8, f"Expected 8 chunks, got {len(chunks)}"

        # Test specific queries with expected results
        test_cases = [
            (
                "How long should I cook spaghetti?",
                "cooking",
                0.60,
            ),  # Should match cooking content with high similarity
            (
                "Which planet is closest to the Sun?",
                "solar_system",
                0.50,
            ),  # Should match solar system content
            (
                "When should I plant tomatoes?",
                "gardening",
                0.35,
            ),  # Should match gardening content
        ]

        for query, expected_topic, min_similarity in test_cases:
            # Generate query embedding
            query_result = generate_embeddings_sync(
                texts=[query], task=QUERY_TASK, chunk_size=512, chunking_mode="none"
            )
            query_embedding = torch.tensor(query_result[0][0].embedding).unsqueeze(0)

            # Compute similarities with all chunks
            similarities = torch.matmul(query_embedding, chunk_embeddings.T).squeeze()

            # Get top 3 chunks
            top_k_indices = torch.topk(similarities, k=3).indices
            top_k_similarities = similarities[top_k_indices]

            # Get content of top matching chunks
            top_chunks_content = []
            for chunk_idx in top_k_indices:
                start, end = chunk_spans[chunk_idx]
                chunk_content = diverse_document[start:end]
                top_chunks_content.append(chunk_content)

            print(f"\nQuery: {query}")
            print(f"Expected topic: {expected_topic}")
            print("Top 3 matching chunks:")
            for i, (chunk_idx, similarity) in enumerate(
                zip(top_k_indices, top_k_similarities)
            ):
                print(f"  {i+1}. Similarity: {similarity:.3f}")
                print(f"     Content: {top_chunks_content[i]}")
            print("-" * 80)

            # Assert that top similarity meets minimum threshold
            assert (
                top_k_similarities[0].item() >= min_similarity
            ), f"Query '{query}' top similarity {top_k_similarities[0].item():.3f} below threshold {min_similarity}"

            # Assert that top chunk contains expected topic content
            top_chunk_content = top_chunks_content[0].lower()
            if expected_topic == "cooking":
                assert any(
                    word in top_chunk_content
                    for word in ["pasta", "cook", "boil", "drain"]
                ), f"Top chunk for cooking query doesn't contain cooking terms: {top_chunk_content[:100]}"
            elif expected_topic == "solar_system":
                assert any(
                    word in top_chunk_content
                    for word in ["solar", "planet", "sun", "mercury", "jupiter"]
                ), f"Top chunk for solar system query doesn't contain space terms: {top_chunk_content[:100]}"
                pass
            elif expected_topic == "gardening":
                assert any(
                    word in top_chunk_content
                    for word in ["garden", "plant", "soil", "crop", "tomato"]
                ), f"Top chunk for gardening query doesn't contain gardening terms: {top_chunk_content[:100]}"
                pass

    def test_semantic_search_with_larger_chunks(self, diverse_document):
        """Test semantic search with larger chunk sizes."""
        # Generate embeddings with larger chunks
        doc_results = generate_embeddings_sync(
            texts=[diverse_document],
            task=PASSAGE_TASK,
            chunk_size=400,  # Larger chunks
            chunking_mode="sentences",
        )

        # Test with a simple query
        query = "How long should I cook pasta?"
        query_result = generate_embeddings_sync(
            texts=[query], task=QUERY_TASK, chunk_size=512, chunking_mode="none"
        )

        chunks = doc_results[0]
        chunk_embeddings = torch.tensor([chunk.embedding for chunk in chunks])
        query_embedding = torch.tensor(query_result[0][0].embedding).unsqueeze(0)

        similarities = torch.matmul(query_embedding, chunk_embeddings.T).squeeze()
        best_chunk_idx = torch.argmax(similarities).item()
        best_similarity = similarities[best_chunk_idx].item()

        print(f"DEBUG: Larger chunks test - got {len(chunks)} chunks")
        print(f"DEBUG: Best similarity: {best_similarity:.3f}")

        # With larger chunks, should get fewer chunks but still good matches
        assert (
            len(chunks) < 8
        ), f"Expected fewer than 8 chunks with larger chunk size, got {len(chunks)}"
        assert len(chunks) > 0, "Should have at least one chunk"

        # Should still find relevant cooking content
        assert (
            best_similarity > 0.5
        ), f"Expected similarity > 0.5 for cooking query, got {best_similarity:.3f}"

        # Verify the best matching chunk contains cooking content
        best_chunk_span = chunks[best_chunk_idx].span
        best_chunk_content = diverse_document[
            best_chunk_span[0] : best_chunk_span[1]
        ].lower()
        assert any(
            word in best_chunk_content for word in ["pasta", "cook", "boil"]
        ), f"Best chunk doesn't contain cooking terms: {best_chunk_content[:100]}"

    def test_embedding_consistency(self):
        """Test that same inputs produce consistent embeddings."""
        text = "This is a test for embedding consistency."

        # Generate embeddings twice
        result1 = generate_embeddings_sync(
            texts=[text], task=PASSAGE_TASK, chunk_size=512, chunking_mode="none"
        )

        result2 = generate_embeddings_sync(
            texts=[text], task=PASSAGE_TASK, chunk_size=512, chunking_mode="none"
        )

        embedding1 = result1[0][0].embedding
        embedding2 = result2[0][0].embedding

        # Embeddings should be identical for the same input
        assert np.allclose(embedding1, embedding2, atol=1e-8)


class TestEdgeCases:
    """Test edge cases and error handling."""

    def test_empty_text(self):
        """Test handling of empty text."""
        result = generate_embeddings_sync(
            texts=[""], task=PASSAGE_TASK, chunk_size=512, chunking_mode="none"
        )

        assert len(result) == 1
        assert len(result[0]) == 1
        assert isinstance(result[0][0], Chunk)

    def test_very_short_text(self):
        """Test handling of very short text."""
        result = generate_embeddings_sync(
            texts=["Hi"], task=PASSAGE_TASK, chunk_size=512, chunking_mode="sentences"
        )

        assert len(result) == 1
        assert len(result[0]) >= 1

    def test_long_text_chunking(self):
        """Test handling of very long text that exceeds model limits."""
        # Create a very long text (repeat to ensure it's longer than MAX_MODEL_SEQ_LEN)
        long_text = "This is a sentence. " * 1000

        result = generate_embeddings_sync(
            texts=[long_text],
            task=PASSAGE_TASK,
            chunk_size=200,
            chunking_mode="sentences",
        )

        assert len(result) == 1
        assert (
            len(result[0]) == 225
        )  # 1000 sentences with chunk_size=200 produces 225 chunks

    def test_special_characters(self):
        """Test handling of text with special characters."""
        special_text = (
            "Text with émojis 🤖 and special chars: @#$%^&*()[]{}|\\:;\"'<>,.?/~`"
        )

        result = generate_embeddings_sync(
            texts=[special_text],
            task=PASSAGE_TASK,
            chunk_size=512,
            chunking_mode="none",
        )

        assert len(result) == 1
        assert len(result[0]) == 1
        assert isinstance(result[0][0].embedding, list)
        assert len(result[0][0].embedding) > 0


class TestPerformance:
    """Test performance and consistency."""

    def test_chunk_span_alignment(self, model_and_tokenizer):
        """Test that character and token spans are properly aligned."""
        _, tokenizer = model_and_tokenizer
        text = "First sentence. Second sentence. Third sentence."
        inputs = tokenize(tokenizer, [text])

        char_spans, token_spans = chunk_by_sentences(inputs, tokenizer, chunk_size=50)

        # Verify that char and token spans correspond
        assert len(char_spans[0]) == len(token_spans[0])

        # For each chunk, verify that the character span makes sense
        for (char_start, char_end), (token_start, token_end) in zip(
            char_spans[0], token_spans[0]
        ):
            chunk_text = text[char_start:char_end]
            assert len(chunk_text) > 0
            assert chunk_text.strip()  # Should not be just whitespace

            # Token spans should be valid indices
            assert 0 <= token_start < token_end
            assert token_end <= inputs.input_ids.shape[1]

    def test_embedding_dimensions(self):
        """Test that all embeddings have consistent dimensions."""
        texts = [
            "Short.",
            "A longer sentence with more words.",
            "Text with different length and complexity.",
        ]

        result = generate_embeddings_sync(
            texts=texts, task=PASSAGE_TASK, chunk_size=100, chunking_mode="sentences"
        )

        # All embeddings should have the same dimension
        embedding_dims = set()
        for text_chunks in result:
            for chunk in text_chunks:
                embedding_dims.add(len(chunk.embedding))

        assert (
            len(embedding_dims) == 1
        ), f"Inconsistent embedding dimensions: {embedding_dims}"

    def test_batch_processing_consistency(self):
        """Test that batch processing produces same results as individual processing."""
        texts = ["First text.", "Second text."]

        # Process as batch
        batch_result = generate_embeddings_sync(
            texts=texts, task=PASSAGE_TASK, chunk_size=512, chunking_mode="none"
        )

        # Process individually
        individual_results = []
        for text in texts:
            individual_result = generate_embeddings_sync(
                texts=[text], task=PASSAGE_TASK, chunk_size=512, chunking_mode="none"
            )
            individual_results.append(individual_result[0])

        # Results should be identical
        assert len(batch_result) == len(individual_results)
        for batch_chunks, individual_chunks in zip(batch_result, individual_results):
            assert len(batch_chunks) == len(individual_chunks)
            for batch_chunk, individual_chunk in zip(batch_chunks, individual_chunks):
                assert np.allclose(
                    batch_chunk.embedding, individual_chunk.embedding, atol=1e-8
                )


if __name__ == "__main__":
    # Run tests
    pytest.main([__file__, "-v"])

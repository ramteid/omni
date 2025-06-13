#!/usr/bin/env python3
"""
Test script for the embeddings API

Usage:
    # Run tests with automatic server startup/shutdown
    python test_embeddings.py
    
    # Use an already-running server
    python test_embeddings.py --use-existing
    
    # Use a custom server URL (with --use-existing)
    AI_SERVICE_URL=http://localhost:8001 python test_embeddings.py --use-existing
"""
import requests
import json
import numpy as np
import subprocess
import time
import sys
import os
import signal
from contextlib import contextmanager
import code


@contextmanager
def test_server(port=8000, max_retries=30):
    """Context manager to start and stop the AI service for testing"""
    # Change to the AI service directory
    ai_service_dir = os.path.join(os.path.dirname(__file__), "..")
    original_dir = os.getcwd()
    os.chdir(ai_service_dir)
    
    # Start the server
    print(f"Starting AI service on port {port}...")
    server_process = subprocess.Popen(
        [sys.executable, "-m", "uvicorn", "main:app", "--host", "0.0.0.0", "--port", str(port)],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        preexec_fn=os.setsid if sys.platform != "win32" else None
    )
    
    # Wait for server to be ready
    health_url = f"http://localhost:{port}/health"
    for i in range(max_retries):
        try:
            response = requests.get(health_url, timeout=1)
            if response.status_code == 200:
                print(f"✓ AI service is ready (took {i+1} seconds)")
                break
        except (requests.exceptions.ConnectionError, requests.exceptions.Timeout):
            pass
        time.sleep(1)
    else:
        # Clean up if server didn't start
        if sys.platform != "win32":
            os.killpg(os.getpgid(server_process.pid), signal.SIGTERM)
        else:
            server_process.terminate()
        server_process.wait()
        os.chdir(original_dir)
        raise RuntimeError(f"AI service failed to start after {max_retries} seconds")
    
    try:
        yield f"http://localhost:{port}"
    finally:
        # Stop the server
        print("\nStopping AI service...")
        if sys.platform != "win32":
            os.killpg(os.getpgid(server_process.pid), signal.SIGTERM)
        else:
            server_process.terminate()
        
        # Wait for graceful shutdown
        try:
            server_process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            # Force kill if necessary
            if sys.platform != "win32":
                os.killpg(os.getpgid(server_process.pid), signal.SIGKILL)
            else:
                server_process.kill()
            server_process.wait()
        
        os.chdir(original_dir)
        print("✓ AI service stopped")


def test_embeddings_api(base_url):
    """Test the embeddings endpoint with both chunking modes"""

    # Test data - same as in the provided example but with multiple sentences
    test_text = """Alice was beginning to get very tired of sitting by her sister on the bank, and of having nothing to do: once or twice she had peeped into the book her sister was reading, but it had no pictures or conversations in it. 'And what is the use of a book,' thought Alice 'without pictures or conversation?' So she was considering in her own mind (as well as she could, for the hot day made her feel very sleepy and stupid), whether the pleasure of making a daisy-chain would be worth the trouble of getting up and picking the daisies, when suddenly a White Rabbit with pink eyes ran close by her."""

    url = f"{base_url}/embeddings"

    # Test sentence-based chunking
    print("Testing sentence-based chunking...")
    payload_sentence = {
        "texts": [test_text],
        "task": "retrieval.passage",
        "chunking_mode": "sentence",
    }

    try:
        response = requests.post(url, json=payload_sentence)
        if response.status_code != 200:
            print(f"Error response: {response.text}")
        assert response.status_code == 200, f"Expected status 200, got {response.status_code}"
        
        result = response.json()
        
        # Assert response structure
        assert "embeddings" in result, "Response should contain 'embeddings' field"
        assert "chunks_count" in result, "Response should contain 'chunks_count' field"
        
        # Assert embeddings properties
        chunk_embeddings = result["embeddings"][0]
        assert len(chunk_embeddings) > 0, "Should return at least one embedding"
        assert len(chunk_embeddings) == result["chunks_count"][0], f"Number of embeddings ({len(chunk_embeddings)}) should match chunks_count ({result['chunks_count']})"
        
        # Assert embedding dimensions (e5-large-v2 produces 1024-dimensional embeddings)
        for i, embedding in enumerate(chunk_embeddings):
            assert len(embedding) == 1024, f"Embedding {i} should have 1024 dimensions, got {len(embedding)}"
            assert isinstance(embedding, list), f"Embedding {i} should be a list"
            assert all(isinstance(x, (int, float)) for x in embedding), f"Embedding {i} should contain only numbers"
        
        # Assert embeddings are normalized (L2 norm should be close to 1)
        for i, embedding in enumerate(chunk_embeddings):
            norm = np.linalg.norm(embedding)
            assert 0.99 < norm < 1.01, f"Embedding {i} L2 norm should be close to 1, got {norm}"
        
        print(f"✓ Sentence chunking passed! Generated {len(chunk_embeddings)} embeddings")
        print(f"✓ Each embedding has {len(chunk_embeddings[0])} dimensions")
        print(f"✓ All embeddings are properly normalized")

    except requests.exceptions.ConnectionError:
        raise AssertionError("Connection error - AI service is not responding")
    except Exception as e:
        raise AssertionError(f"Sentence chunking error: {str(e)}")

    print()

    # Test fixed-size chunking
    print("Testing fixed-size chunking...")
    payload_fixed = {
        "texts": [test_text],
        "task": "retrieval.passage",
        "chunking_mode": "fixed",
        "chunk_size": 512,
    }

    try:
        response = requests.post(url, json=payload_fixed)
        if response.status_code != 200:
            print(f"Error response: {response.text}")
        assert response.status_code == 200, f"Expected status 200, got {response.status_code}"
        
        result = response.json()
        
        # Assert response structure
        assert "embeddings" in result, "Response should contain 'embeddings' field"
        assert "chunks_count" in result, "Response should contain 'chunks_count' field"
        
        # Assert embeddings properties
        chunk_embeddings = result["embeddings"][0]
        assert len(chunk_embeddings) > 0, "Should return at least one embedding"
        assert len(chunk_embeddings) == result["chunks_count"][0], f"Number of embeddings ({len(chunk_embeddings)}) should match chunks_count ({result['chunks_count']})"
        
        # Assert embedding dimensions
        for i, embedding in enumerate(chunk_embeddings):
            assert len(embedding) == 1024, f"Embedding {i} should have 1024 dimensions, got {len(embedding)}"
            assert isinstance(embedding, list), f"Embedding {i} should be a list"
            assert all(isinstance(x, (int, float)) for x in embedding), f"Embedding {i} should contain only numbers"
        
        # Assert embeddings are normalized
        for i, embedding in enumerate(chunk_embeddings):
            norm = np.linalg.norm(embedding)
            assert 0.99 < norm < 1.01, f"Embedding {i} L2 norm should be close to 1, got {norm}"
        
        print(f"✓ Fixed chunking passed! Generated {len(chunk_embeddings)} embeddings")
        print(f"✓ Each embedding has {len(chunk_embeddings[0])} dimensions")
        print(f"✓ All embeddings are properly normalized")

    except requests.exceptions.ConnectionError:
        raise AssertionError("Connection error - AI service is not responding")
    except Exception as e:
        raise AssertionError(f"Fixed chunking error: {str(e)}")


def test_health_endpoint(base_url):
    """Test the health endpoint"""
    try:
        response = requests.get(f"{base_url}/health")
        assert response.status_code == 200, f"Expected status 200, got {response.status_code}"
        
        result = response.json()
        assert "status" in result, "Health response should contain 'status' field"
        assert result["status"] == "healthy", f"Expected status 'healthy', got '{result['status']}'"
        
        print(f"✓ Health check passed: {result}")
    except Exception as e:
        raise AssertionError(f"Health check error: {str(e)}")


def test_edge_cases(base_url):
    """Test edge cases and error handling"""
    url = f"{base_url}/embeddings"
    
    print("Testing edge cases...")
    
    # Test empty text
    print("Testing empty text...")
    payload = {
        "texts": [""],
        "task": "retrieval.passage",
        "chunking_mode": "sentence",
    }
    response = requests.post(url, json=payload)
    assert response.status_code == 200, "Should handle empty text gracefully"
    result = response.json()
    assert len(result["embeddings"][0]) == 0, "Empty text should produce no embeddings"
    print("✓ Empty text handled correctly")
    
    # Test multiple texts
    print("Testing multiple texts...")
    payload = {
        "texts": ["First text.", "Second text.", "Third text."],
        "task": "retrieval.passage",
        "chunking_mode": "sentence",
    }
    response = requests.post(url, json=payload)
    assert response.status_code == 200, "Should handle multiple texts"
    result = response.json()
    assert len(result["embeddings"]) == 3, "Should produce exactly one embedding per text"
    assert all([len(chunks) > 0 for chunks in result["embeddings"]]), "Should product at least one chunk per text"
    print(f"✓ Multiple texts handled correctly: {len(result['embeddings'])} embeddings")
    
    # Test invalid chunking mode
    print("Testing invalid chunking mode...")
    payload = {
        "texts": ["Test text"],
        "task": "retrieval.passage",
        "chunking_mode": "invalid",
    }
    response = requests.post(url, json=payload)
    assert response.status_code == 422, "Should reject invalid chunking mode"
    print("✓ Invalid chunking mode rejected correctly")
    
    # Test missing required fields
    print("Testing missing required fields...")
    payload = {
        # Missing texts
    }
    response = requests.post(url, json=payload)
    assert response.status_code == 422, "Should reject request with missing required fields"
    print("✓ Missing required fields rejected correctly")


if __name__ == "__main__":
    print("Testing Clio AI Service Embeddings API")
    print("=" * 50)
    
    # Check if user wants to use an existing server
    use_existing = "--use-existing" in sys.argv
    base_url = os.environ.get("AI_SERVICE_URL", "http://localhost:8000")

    try:
        if use_existing:
            print(f"Using existing AI service at {base_url}")
            test_health_endpoint(base_url)
            print()
            print("Comparing sentence-based vs fixed-size chunking:")
            print("-" * 50)
            test_embeddings_api(base_url)
            print()
            test_edge_cases(base_url)
        else:
            # Use context manager to start/stop server automatically
            with test_server() as base_url:
                test_health_endpoint(base_url)
                print()
                print("Comparing sentence-based vs fixed-size chunking:")
                print("-" * 50)
                test_embeddings_api(base_url)
                print()
                test_edge_cases(base_url)
        
        print("\n" + "=" * 50)
        print("All tests passed! ✅")
        print("\nKey differences:")
        print(
            "• Sentence chunking: Respects natural sentence boundaries for more semantic coherence"
        )
        print("• Fixed chunking: Uses fixed token windows, may split sentences mid-way")
        print(
            "• Sentence chunking typically produces more variable chunk counts based on text structure"
        )
    except AssertionError as e:
        print(f"\n❌ Test failed: {e}")
        exit(1)
    except Exception as e:
        print(f"\n❌ Unexpected error: {e}")
        exit(1)

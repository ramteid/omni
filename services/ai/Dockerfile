# Build stage
FROM python:3.12-slim AS builder

ENV PYTHONDONTWRITEBYTECODE=1
ENV PYTHONUNBUFFERED=1

# Install build dependencies
RUN apt-get update && apt-get install -y \
    gcc \
    g++ \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy requirements and create CPU-only version
COPY requirements.txt requirements-original.txt

# Create CPU-only requirements by removing GPU-specific packages and torch
RUN grep -v -E "(flash-attn|nvidia-|triton|^torch==)" requirements-original.txt > requirements-cpu.txt

# Install CPU-only PyTorch first
RUN pip install --no-cache-dir torch==2.7.1 torchvision --index-url https://download.pytorch.org/whl/cpu

# Install remaining dependencies
RUN pip install --no-cache-dir -r requirements-cpu.txt

# Runtime stage
FROM python:3.12-slim AS runtime

ENV PYTHONDONTWRITEBYTECODE=1
ENV PYTHONUNBUFFERED=1

# Install only runtime dependencies
RUN apt-get update && apt-get install -y \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy Python packages from builder
COPY --from=builder /usr/local/lib/python3.12/site-packages /usr/local/lib/python3.12/site-packages
COPY --from=builder /usr/local/bin /usr/local/bin

# Copy application code
COPY . .

# Run the application
# PORT and AI_WORKERS must be provided at runtime via environment variables
CMD sh -c "python -m uvicorn main:app --host 0.0.0.0 --port ${PORT} --workers ${AI_WORKERS:-1}"
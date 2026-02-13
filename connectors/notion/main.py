#!/usr/bin/env python3
"""Notion Connector entry point for Omni."""

import logging
import os

from notion_connector import NotionConnector

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
)

if __name__ == "__main__":
    port = int(os.environ.get("PORT", "8000"))
    NotionConnector().serve(port=port)

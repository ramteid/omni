#!/usr/bin/env python3
"""HubSpot Connector entry point for Omni."""

import logging
import os

from hubspot_connector import HubSpotConnector

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
)

if __name__ == "__main__":
    port = os.environ.get("PORT")
    if not port:
        raise SystemExit("PORT environment variable is required")
    port = int(port)
    HubSpotConnector().serve(port=port)

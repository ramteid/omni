"""Integration tests: sync continues when some object types return 403 (missing scopes)."""

import pytest
import httpx

from omni_connector.testing import count_events, wait_for_sync

pytestmark = pytest.mark.integration


async def test_sync_skips_forbidden_object_types(
    harness, seed, source_id, mock_hubspot_api, cm_client: httpx.AsyncClient
):
    """Sync should complete successfully even when some object types return 403."""
    mock_hubspot_api.forbidden_types = {"tickets", "deals"}

    mock_hubspot_api.add_contact("101")
    mock_hubspot_api.add_company("201")

    resp = await cm_client.post(
        "/sync",
        json={"source_id": source_id, "sync_type": "full"},
    )
    assert resp.status_code == 200, resp.text
    sync_run_id = resp.json()["sync_run_id"]

    row = await wait_for_sync(harness.db_pool, sync_run_id, timeout=30)
    assert (
        row["status"] == "completed"
    ), f"Sync ended with status={row['status']}, error={row.get('error_message')}"

    n_events = await count_events(harness.db_pool, source_id, "document_created")
    assert n_events >= 2, f"Expected >=2 document_created events, got {n_events}"

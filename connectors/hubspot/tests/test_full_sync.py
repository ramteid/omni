"""Integration tests: full sync creates all document types."""

import pytest
import httpx

from omni_connector.testing import count_events, wait_for_sync

pytestmark = pytest.mark.integration


async def test_full_sync_creates_all_documents(
    harness, seed, source_id, mock_hubspot_api, cm_client: httpx.AsyncClient
):
    mock_hubspot_api.add_contact("101")
    mock_hubspot_api.add_company("201")
    mock_hubspot_api.add_deal("301")
    mock_hubspot_api.add_ticket("401")
    mock_hubspot_api.add_call("501")
    mock_hubspot_api.add_email("601")
    mock_hubspot_api.add_meeting("701")
    mock_hubspot_api.add_note("801")
    mock_hubspot_api.add_task("901")

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
    assert n_events >= 9, f"Expected >=9 document_created events, got {n_events}"


async def test_full_sync_scanned_count(
    harness, seed, source_id, mock_hubspot_api, cm_client: httpx.AsyncClient
):
    mock_hubspot_api.add_contact("111")
    mock_hubspot_api.add_contact("112")
    mock_hubspot_api.add_company("211")
    mock_hubspot_api.add_deal("311")

    resp = await cm_client.post(
        "/sync",
        json={"source_id": source_id, "sync_type": "full"},
    )
    sync_run_id = resp.json()["sync_run_id"]
    row = await wait_for_sync(harness.db_pool, sync_run_id, timeout=30)

    assert row["status"] == "completed"
    assert (
        row["documents_scanned"] >= 4
    ), f"Expected >=4 documents_scanned, got {row['documents_scanned']}"


async def test_full_sync_with_portal_id(
    harness, seed, source_id, mock_hubspot_api, cm_client: httpx.AsyncClient
):
    mock_hubspot_api.add_contact("121", firstname="Alice", lastname="Smith")

    resp = await cm_client.post(
        "/sync",
        json={"source_id": source_id, "sync_type": "full"},
    )
    sync_run_id = resp.json()["sync_run_id"]

    row = await wait_for_sync(harness.db_pool, sync_run_id, timeout=30)
    assert row["status"] == "completed"

    n_events = await count_events(harness.db_pool, source_id, "document_created")
    assert n_events >= 1, f"Expected >=1 document_created events, got {n_events}"

"""Integration tests: full sync indexes pages and databases."""

import pytest
import httpx

from omni_connector.testing import count_events, get_events, wait_for_sync

from .conftest import _block_payload

pytestmark = pytest.mark.integration

DB_ID = "db-00000000-0000-0000-0000-000000000001"
ENTRY_PAGE_ID = "pg-00000000-0000-0000-0000-000000000001"
STANDALONE_PAGE_ID = "pg-00000000-0000-0000-0000-000000000002"


async def test_full_sync_indexes_pages_and_databases(
    harness, seed, source_id, mock_notion_api, cm_client: httpx.AsyncClient
):
    properties_schema = {
        "Name": {"id": "title", "name": "Name", "type": "title", "title": {}},
        "Status": {
            "id": "status",
            "name": "Status",
            "type": "select",
            "select": {
                "options": [
                    {"name": "Not Started", "color": "default"},
                    {"name": "In Progress", "color": "blue"},
                    {"name": "Done", "color": "green"},
                ]
            },
        },
        "Priority": {
            "id": "priority",
            "name": "Priority",
            "type": "select",
            "select": {
                "options": [
                    {"name": "High", "color": "red"},
                    {"name": "Medium", "color": "yellow"},
                    {"name": "Low", "color": "default"},
                ]
            },
        },
    }
    mock_notion_api.add_database(
        DB_ID, "Project Tasks", properties_schema, description="Team task tracker"
    )

    entry_properties = {
        "Name": {
            "id": "title",
            "type": "title",
            "title": [
                {
                    "type": "text",
                    "text": {"content": "Fix login bug"},
                    "plain_text": "Fix login bug",
                }
            ],
        },
        "Status": {
            "id": "status",
            "type": "select",
            "select": {"name": "In Progress", "color": "blue"},
        },
        "Priority": {
            "id": "priority",
            "type": "select",
            "select": {"name": "High", "color": "red"},
        },
    }
    entry_blocks = [
        _block_payload(
            "blk-001", "paragraph", "This bug causes the login form to crash on submit."
        ),
        _block_payload("blk-002", "heading_2", "Steps to Reproduce"),
    ]
    mock_notion_api.add_database_entry(
        DB_ID, ENTRY_PAGE_ID, "Fix login bug", entry_properties, entry_blocks
    )

    standalone_blocks = [
        _block_payload("blk-010", "paragraph", "Welcome to the engineering handbook."),
        _block_payload("blk-011", "heading_1", "Onboarding"),
        _block_payload(
            "blk-012", "bulleted_list_item", "Set up your development environment"
        ),
    ]
    mock_notion_api.add_page(
        STANDALONE_PAGE_ID, "Engineering Handbook", standalone_blocks
    )

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
    assert n_events == 3, f"Expected 3 document_created events, got {n_events}"

    events = await get_events(harness.db_pool, source_id)
    doc_ids = {
        e["payload"]["document_id"]
        for e in events
        if e["event_type"] == "document_created"
    }
    assert f"notion:database:{DB_ID}" in doc_ids
    assert f"notion:page:{ENTRY_PAGE_ID}" in doc_ids
    assert f"notion:page:{STANDALONE_PAGE_ID}" in doc_ids

    state = await seed.get_connector_state(source_id)
    assert state is not None, "connector_state should be saved after sync"
    assert "last_sync_at" in state

    assert (
        row["documents_scanned"] >= 3
    ), f"Expected >=3 documents_scanned, got {row['documents_scanned']}"

"""Integration test fixtures for the HubSpot connector.

Session-scoped: harness, mock HubSpot API server, connector server, connector-manager.
Function-scoped: seed helper, source_id, httpx client.
"""

from __future__ import annotations

import logging
import socket
import threading
import time
from typing import Any

import httpx
import pytest
import pytest_asyncio
import uvicorn
from starlette.applications import Starlette
from starlette.requests import Request
from starlette.responses import JSONResponse
from starlette.routing import Route

from omni_connector.testing import OmniTestHarness, SeedHelper

logger = logging.getLogger(__name__)


# ---------------------------------------------------------------------------
# Mock data helpers
# ---------------------------------------------------------------------------


def _object_payload(
    object_type: str,
    obj_id: str,
    properties: dict[str, Any],
) -> dict[str, Any]:
    return {
        "id": obj_id,
        "properties": {
            **properties,
            "hs_object_id": obj_id,
        },
        "createdAt": properties.get("createdate", "2024-01-15T10:30:00.000Z"),
        "updatedAt": properties.get("hs_lastmodifieddate", "2024-06-01T14:00:00.000Z"),
        "archived": False,
    }


def _contact_payload(
    obj_id: str = "101",
    firstname: str = "John",
    lastname: str = "Doe",
    email: str = "john@example.com",
) -> dict[str, Any]:
    return _object_payload(
        "contacts",
        obj_id,
        {
            "firstname": firstname,
            "lastname": lastname,
            "email": email,
            "phone": "+1234567890",
            "company": "Acme Corp",
            "jobtitle": "Engineer",
            "lifecyclestage": "lead",
            "createdate": "2024-01-15T10:30:00.000Z",
            "hs_lastmodifieddate": "2024-06-01T14:00:00.000Z",
            "hubspot_owner_id": "owner-1",
        },
    )


def _company_payload(
    obj_id: str = "201",
    name: str = "Acme Corp",
    domain: str = "acme.com",
) -> dict[str, Any]:
    return _object_payload(
        "companies",
        obj_id,
        {
            "name": name,
            "domain": domain,
            "industry": "Technology",
            "phone": "+1987654321",
            "numberofemployees": "50",
            "annualrevenue": "1000000",
            "createdate": "2024-01-10T08:00:00.000Z",
            "hs_lastmodifieddate": "2024-05-20T12:00:00.000Z",
            "hubspot_owner_id": "owner-2",
        },
    )


def _deal_payload(
    obj_id: str = "301",
    dealname: str = "Big Deal",
) -> dict[str, Any]:
    return _object_payload(
        "deals",
        obj_id,
        {
            "dealname": dealname,
            "amount": "50000",
            "pipeline": "default",
            "dealstage": "qualifiedtobuy",
            "closedate": "2024-12-31T00:00:00.000Z",
            "hs_deal_stage_probability": "0.5",
            "createdate": "2024-02-01T09:00:00.000Z",
            "hs_lastmodifieddate": "2024-06-10T11:00:00.000Z",
            "hubspot_owner_id": "owner-1",
        },
    )


def _ticket_payload(
    obj_id: str = "401",
    subject: str = "Support request",
) -> dict[str, Any]:
    return _object_payload(
        "tickets",
        obj_id,
        {
            "subject": subject,
            "content": "Need help with integration",
            "hs_ticket_priority": "HIGH",
            "hs_pipeline": "0",
            "hs_pipeline_stage": "1",
            "createdate": "2024-03-01T10:00:00.000Z",
            "hs_lastmodifieddate": "2024-03-15T16:00:00.000Z",
            "hubspot_owner_id": "owner-3",
        },
    )


def _call_payload(obj_id: str = "501") -> dict[str, Any]:
    return _object_payload(
        "calls",
        obj_id,
        {
            "hs_call_title": "Discovery call",
            "hs_call_body": "Discussed requirements",
            "hs_call_duration": "1800000",
            "hs_call_direction": "OUTBOUND",
            "hs_timestamp": "2024-04-01T14:00:00.000Z",
            "hs_createdate": "2024-04-01T14:00:00.000Z",
            "hs_lastmodifieddate": "2024-04-01T15:00:00.000Z",
            "hubspot_owner_id": "owner-1",
        },
    )


def _email_payload(obj_id: str = "601") -> dict[str, Any]:
    return _object_payload(
        "emails",
        obj_id,
        {
            "hs_email_subject": "Follow up",
            "hs_email_text": "Thanks for the call",
            "hs_email_html": "<p>Thanks for the call</p>",
            "hs_email_direction": "EMAIL",
            "hs_timestamp": "2024-04-02T09:00:00.000Z",
            "hs_createdate": "2024-04-02T09:00:00.000Z",
            "hs_lastmodifieddate": "2024-04-02T09:30:00.000Z",
            "hubspot_owner_id": "owner-1",
        },
    )


def _meeting_payload(obj_id: str = "701") -> dict[str, Any]:
    return _object_payload(
        "meetings",
        obj_id,
        {
            "hs_meeting_title": "Kickoff meeting",
            "hs_meeting_body": "Project kickoff",
            "hs_meeting_start_time": "2024-04-10T10:00:00.000Z",
            "hs_meeting_end_time": "2024-04-10T11:00:00.000Z",
            "hs_timestamp": "2024-04-10T10:00:00.000Z",
            "hs_createdate": "2024-04-10T10:00:00.000Z",
            "hs_lastmodifieddate": "2024-04-10T11:30:00.000Z",
            "hubspot_owner_id": "owner-2",
        },
    )


def _note_payload(obj_id: str = "801") -> dict[str, Any]:
    return _object_payload(
        "notes",
        obj_id,
        {
            "hs_note_body": "Important note about the deal",
            "hs_timestamp": "2024-04-15T08:00:00.000Z",
            "hs_createdate": "2024-04-15T08:00:00.000Z",
            "hs_lastmodifieddate": "2024-04-15T08:30:00.000Z",
            "hubspot_owner_id": "owner-1",
        },
    )


def _task_payload(obj_id: str = "901") -> dict[str, Any]:
    return _object_payload(
        "tasks",
        obj_id,
        {
            "hs_task_subject": "Send proposal",
            "hs_task_body": "Prepare and send the proposal",
            "hs_task_status": "NOT_STARTED",
            "hs_task_priority": "HIGH",
            "hs_timestamp": "2024-04-20T09:00:00.000Z",
            "hs_createdate": "2024-04-20T09:00:00.000Z",
            "hs_lastmodifieddate": "2024-04-20T09:00:00.000Z",
            "hubspot_owner_id": "owner-1",
        },
    )


# ---------------------------------------------------------------------------
# Mock HubSpot API
# ---------------------------------------------------------------------------


class MockHubSpotAPI:
    """Controllable mock of the HubSpot CRM v3 API."""

    def __init__(self) -> None:
        self.objects: dict[str, list[dict[str, Any]]] = {}
        self.should_fail_auth: bool = False
        self.forbidden_types: set[str] = set()

    def reset(self) -> None:
        self.objects.clear()
        self.should_fail_auth = False
        self.forbidden_types.clear()

    def add_contact(self, obj_id: str = "101", **kwargs: Any) -> None:
        self.objects.setdefault("contacts", []).append(
            _contact_payload(obj_id, **kwargs)
        )

    def add_company(self, obj_id: str = "201", **kwargs: Any) -> None:
        self.objects.setdefault("companies", []).append(
            _company_payload(obj_id, **kwargs)
        )

    def add_deal(self, obj_id: str = "301", **kwargs: Any) -> None:
        self.objects.setdefault("deals", []).append(_deal_payload(obj_id, **kwargs))

    def add_ticket(self, obj_id: str = "401", **kwargs: Any) -> None:
        self.objects.setdefault("tickets", []).append(_ticket_payload(obj_id, **kwargs))

    def add_call(self, obj_id: str = "501") -> None:
        self.objects.setdefault("calls", []).append(_call_payload(obj_id))

    def add_email(self, obj_id: str = "601") -> None:
        self.objects.setdefault("emails", []).append(_email_payload(obj_id))

    def add_meeting(self, obj_id: str = "701") -> None:
        self.objects.setdefault("meetings", []).append(_meeting_payload(obj_id))

    def add_note(self, obj_id: str = "801") -> None:
        self.objects.setdefault("notes", []).append(_note_payload(obj_id))

    def add_task(self, obj_id: str = "901") -> None:
        self.objects.setdefault("tasks", []).append(_task_payload(obj_id))

    def create_app(self) -> Starlette:
        mock = self

        async def list_objects(request: Request) -> JSONResponse:
            if mock.should_fail_auth:
                return JSONResponse(
                    {
                        "status": "error",
                        "message": "Authentication credentials not found",
                        "category": "INVALID_AUTHENTICATION",
                    },
                    status_code=401,
                )

            object_type = request.path_params["object_type"]

            if object_type in mock.forbidden_types:
                return JSONResponse(
                    {
                        "status": "error",
                        "message": "This access token does not have proper permissions",
                        "category": "FORBIDDEN",
                    },
                    status_code=403,
                )

            items = mock.objects.get(object_type, [])

            limit = int(request.query_params.get("limit", "100"))
            after = request.query_params.get("after")

            start = int(after) if after else 0
            page = items[start : start + limit]
            has_more = (start + limit) < len(items)

            # Filter properties if requested
            requested_props = request.query_params.get("properties")
            if requested_props:
                prop_list = requested_props.split(",")
                filtered_page = []
                for obj in page:
                    filtered_props = {
                        k: v
                        for k, v in obj["properties"].items()
                        if k in prop_list or k == "hs_object_id"
                    }
                    filtered_page.append({**obj, "properties": filtered_props})
                page = filtered_page

            body: dict[str, Any] = {"results": page}
            if has_more:
                body["paging"] = {"next": {"after": str(start + limit)}}

            return JSONResponse(body)

        routes = [
            Route("/crm/v3/objects/{object_type}", list_objects),
        ]
        return Starlette(routes=routes)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("", 0))
        return s.getsockname()[1]


def _wait_for_port(port: int, host: str = "localhost", timeout: float = 10) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            with socket.create_connection((host, port), timeout=1):
                return
        except OSError:
            time.sleep(0.1)
    raise TimeoutError(f"Port {port} not open after {timeout}s")


# ---------------------------------------------------------------------------
# Session-scoped fixtures
# ---------------------------------------------------------------------------


@pytest.fixture(scope="session")
def mock_hubspot_api() -> MockHubSpotAPI:
    return MockHubSpotAPI()


@pytest.fixture(scope="session")
def mock_hubspot_server(mock_hubspot_api: MockHubSpotAPI) -> str:
    """Start mock HubSpot API server in a daemon thread. Returns base URL."""
    port = _free_port()
    app = mock_hubspot_api.create_app()
    config = uvicorn.Config(app, host="0.0.0.0", port=port, log_level="warning")
    server = uvicorn.Server(config)

    thread = threading.Thread(target=server.run, daemon=True)
    thread.start()

    _wait_for_port(port)
    return f"http://localhost:{port}"


@pytest.fixture(scope="session")
def connector_port() -> int:
    return _free_port()


@pytest.fixture(scope="session")
def connector_server(connector_port: int) -> str:
    """Start the HubSpot connector as a uvicorn server in a daemon thread. Returns base URL."""
    import os

    os.environ.setdefault("CONNECTOR_MANAGER_URL", "http://localhost:0")

    from hubspot_connector import HubSpotConnector
    from omni_connector.server import create_app

    app = create_app(HubSpotConnector())
    config = uvicorn.Config(
        app, host="0.0.0.0", port=connector_port, log_level="warning"
    )
    server = uvicorn.Server(config)

    thread = threading.Thread(target=server.run, daemon=True)
    thread.start()

    _wait_for_port(connector_port)
    return f"http://localhost:{connector_port}"


@pytest_asyncio.fixture(scope="session")
async def harness(
    connector_server: str,
    connector_port: int,
) -> OmniTestHarness:
    """Session-scoped OmniTestHarness with all infrastructure started."""
    import os

    h = OmniTestHarness()
    await h.start_infra()
    await h.start_connector_manager(
        {
            "HUBSPOT_CONNECTOR_URL": f"http://host.docker.internal:{connector_port}",
        }
    )

    os.environ["CONNECTOR_MANAGER_URL"] = h.connector_manager_url

    yield h
    await h.teardown()


# ---------------------------------------------------------------------------
# Function-scoped fixtures
# ---------------------------------------------------------------------------


@pytest_asyncio.fixture
async def seed(harness: OmniTestHarness) -> SeedHelper:
    return harness.seed()


@pytest_asyncio.fixture
async def source_id(
    seed: SeedHelper,
    mock_hubspot_server: str,
    mock_hubspot_api: MockHubSpotAPI,
) -> str:
    """Create a HubSpot source with credentials pointing to the mock server."""
    mock_hubspot_api.reset()
    sid = await seed.create_source(
        source_type="hubspot",
        config={"api_url": mock_hubspot_server, "portal_id": "12345678"},
    )
    await seed.create_credentials(
        sid, {"access_token": "test-token"}, provider="hubspot"
    )
    return sid


@pytest_asyncio.fixture
async def cm_client(harness: OmniTestHarness) -> httpx.AsyncClient:
    """Async httpx client pointed at the connector-manager."""
    async with httpx.AsyncClient(
        base_url=harness.connector_manager_url, timeout=30
    ) as client:
        yield client


# ---------------------------------------------------------------------------
# Fixtures for unit tests (test_mappers.py)
# ---------------------------------------------------------------------------


@pytest.fixture
def mock_hubspot_contact():
    """Create a mock HubSpot contact object."""
    return {
        "id": "123",
        "properties": {
            "firstname": "John",
            "lastname": "Doe",
            "email": "john.doe@example.com",
            "phone": "+1234567890",
            "company": "Acme Corp",
            "jobtitle": "Engineer",
            "createdate": "2024-01-15T10:30:00Z",
            "hs_lastmodifieddate": "2024-06-01T14:00:00Z",
            "hubspot_owner_id": "owner-456",
        },
    }

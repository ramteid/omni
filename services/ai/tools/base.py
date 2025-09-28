"""Base classes for tools."""

class BaseToolHandler:
    """Base class for all tools."""
    pass

class ToolCall:
    """Represents a call to a tool with its name and parameters."""
    def __init__(self, name: str, parameters: dict):
        self.name = name
        self.parameters = parameters

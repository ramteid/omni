"""
AWS Bedrock Provider for Claude models.
"""

import json
import logging
from typing import AsyncIterator, Optional, List, Dict, Any

import boto3
from botocore.exceptions import ClientError
from anthropic.types import (
    ContentBlockDeltaEvent,
    ContentBlockStartEvent,
    ContentBlockStopEvent,
    MessageDeltaEvent,
    ToolUseBlock,
    TextBlock,
    TextDelta,
    InputJSONDelta
)
from anthropic.types.message_stream_event import MessageStreamEvent

from . import LLMProvider

logger = logging.getLogger(__name__)


class BedrockProvider(LLMProvider):
    """Provider for AWS Bedrock Claude models."""

    MODEL_FAMILIES = ["anthropic", "amazon"]

    def __init__(self, model_id: str, region_name: Optional[str] = None):
        self.model_id = model_id
        self.model_family = self._determine_model_family(model_id)
        self.region_name = region_name
        self.client = boto3.client('bedrock-runtime', region_name=region_name)

    def _determine_model_family(self, model_id: str) -> str:
        """Determine the model family from the model ID."""
        for family in self.MODEL_FAMILIES:
            if family in model_id.lower():
                return family
        raise ValueError(f"Unknown model family for model ID: {model_id}")

    async def stream_response(
        self,
        prompt: str,
        max_tokens: Optional[int] = None,
        temperature: Optional[float] = None,
        top_p: Optional[float] = None,
        tools: Optional[List[Dict[str, Any]]] = None,
        messages: Optional[List[Dict[str, Any]]] = None,
    ) -> AsyncIterator[MessageStreamEvent]:
        """Stream response from AWS Bedrock models."""
        try:
            if self.model_family == "anthropic":
                # Use provided messages or create from prompt
                msg_list = messages or [{"role": "user", "content": prompt}]

                # Prepare the request body for Claude models
                body = {
                    "anthropic_version": "bedrock-2023-05-31",
                    "max_tokens": max_tokens or 4096,
                    "messages": msg_list,
                    "temperature": temperature or 0.7,
                    "top_p": top_p or 0.9,
                }

                # Add tools if provided
                if tools:
                    body["tools"] = tools
                    logger.info(f"[BEDROCK] Sending request with {len(tools)} tools: {[t['name'] for t in tools]}")
                else:
                    logger.info(f"[BEDROCK] Sending request without tools")

                logger.info(f"[BEDROCK] Model: {self.model_id}, Messages: {len(msg_list)}, Max tokens: {body['max_tokens']}")
                logger.debug(f"[BEDROCK] Full request body: {json.dumps({k: v for k, v in body.items() if k != 'messages'}, indent=2)}")
                logger.debug(f"[BEDROCK] Messages: {json.dumps(msg_list, indent=2)}")

                # Invoke with streaming response
                logger.info(f"[BEDROCK] Invoking model {self.model_id} with streaming response")
                response = self.client.invoke_model_with_response_stream(
                    modelId=self.model_id,
                    body=json.dumps(body),
                    contentType="application/json",
                    accept="application/json",
                )

                logger.info(f"[BEDROCK] Stream created successfully, processing events")
                event_count = 0
                # Process streaming response and convert to Anthropic-compatible events
                for event in response.get('body'):
                    event_count += 1
                    chunk = json.loads(event['chunk']['bytes'].decode())
                    logger.debug(f"[BEDROCK] Event {event_count}: {chunk.get('type', 'unknown')}")

                    # Convert Bedrock events to Anthropic MessageStreamEvent format
                    if chunk['type'] == 'content_block_start':
                        logger.info(f"[BEDROCK] Content block start: {chunk.get('content_block', {}).get('type', 'unknown')}")
                        if chunk['content_block']['type'] == 'text':
                            content_block = TextBlock(type="text", text="")
                        elif chunk['content_block']['type'] == 'tool_use':
                            tool_id = chunk['content_block']['id']
                            tool_name = chunk['content_block']['name']
                            logger.info(f"[BEDROCK] Tool use started: {tool_name} (id: {tool_id})")
                            content_block = ToolUseBlock(
                                type="tool_use",
                                id=tool_id,
                                name=tool_name,
                                input={}
                            )
                        else:
                            logger.debug(f"[BEDROCK] Skipping unknown content block type: {chunk['content_block']['type']}")
                            continue

                        event_obj = ContentBlockStartEvent(
                            type="content_block_start",
                            index=chunk['index'],
                            content_block=content_block
                        )
                        yield event_obj

                    elif chunk['type'] == 'content_block_delta':
                        if 'text' in chunk['delta']:
                            # Text delta
                            text_content = chunk['delta']['text']
                            logger.debug(f"[BEDROCK] Text delta: {text_content[:50]}...")
                            delta = TextDelta(type="text_delta", text=text_content)
                        elif 'partial_json' in chunk['delta']:
                            # Tool use delta
                            partial_json = chunk['delta']['partial_json']
                            logger.debug(f"[BEDROCK] JSON delta: {partial_json}")
                            delta = InputJSONDelta(type="input_json_delta", partial_json=partial_json)
                        else:
                            logger.debug(f"[BEDROCK] Skipping unknown delta type: {list(chunk['delta'].keys())}")
                            continue

                        event_obj = ContentBlockDeltaEvent(
                            type="content_block_delta",
                            index=chunk['index'],
                            delta=delta
                        )
                        yield event_obj

                    elif chunk['type'] == 'content_block_stop':
                        logger.info(f"[BEDROCK] Content block stop at index {chunk.get('index', 'unknown')}")
                        event_obj = ContentBlockStopEvent(
                            type="content_block_stop",
                            index=chunk['index']
                        )
                        yield event_obj

                    elif chunk['type'] == 'message_delta':
                        if 'stop_reason' in chunk['delta']:
                            stop_reason = chunk['delta']['stop_reason']
                            logger.info(f"[BEDROCK] Message delta - stop reason: {stop_reason}")
                            delta = MessageDelta(stop_reason=stop_reason)
                            event_obj = MessageDeltaEvent(
                                type="message_delta",
                                delta=delta
                            )
                            yield event_obj
                            logger.info(f"[BEDROCK] Stream completed after {event_count} events")
                            break
            elif self.model_family == "amazon":
                logger.info(f"[BEDROCK-AMAZON] Using Amazon model family with model: {self.model_id}")
                logger.debug(f"[BEDROCK-AMAZON] Prompt: {prompt[:100]}...")

                response = self.client.converse_stream(
                    modelId=self.model_id,
                    messages=[{"role": "user", "content": [{"text": prompt}]}],
                    inferenceConfig={
                        "maxTokens": max_tokens or 4096,
                        "temperature": temperature or 0.7,
                        "topP": top_p or 0.9,
                    }
                )

                logger.info(f"[BEDROCK-AMAZON] Stream created, processing chunks")
                chunk_count = 0
                for chunk in response["stream"]:
                    chunk_count += 1
                    logger.debug(f"[BEDROCK-AMAZON] Chunk {chunk_count}: {list(chunk.keys())}")
                    if "contentBlockDelta" in chunk:
                        text = chunk["contentBlockDelta"]["delta"]["text"]
                        logger.debug(f"[BEDROCK-AMAZON] Text chunk: {text[:50]}...")
                        yield text
            else:
                raise ValueError(f"Unsupported model family: {self.model_family}")

        except ClientError as e:
            error_code = e.response.get('Error', {}).get('Code', 'Unknown')
            logger.error(f"[BEDROCK] AWS Bedrock client error ({error_code}): {str(e)}", exc_info=True)
            yield f"Error: AWS Bedrock client error ({error_code})"
        except Exception as e:
            logger.error(f"[BEDROCK] Failed to stream from AWS Bedrock: {str(e)}", exc_info=True)
            yield f"Error: {str(e)}"

    async def generate_response(
        self,
        prompt: str,
        max_tokens: Optional[int] = None,
        temperature: Optional[float] = None,
        top_p: Optional[float] = None,
    ) -> str:
        """Generate non-streaming response from AWS Bedrock Claude models."""
        try:
            # Prepare the request body for Claude models
            body = {
                "anthropic_version": "bedrock-2023-05-31",
                "max_tokens": max_tokens or 4096,
                "messages": [{"role": "user", "content": prompt}],
                "temperature": temperature or 0.7,
                "top_p": top_p or 0.9,
            }

            # Invoke the model
            response = self.client.invoke_model(
                modelId=self.model_id,
                body=json.dumps(body),
                contentType="application/json",
                accept="application/json",
            )

            # Parse response
            response_body = json.loads(response['body'].read())
            content = ""
            for block in response_body.get('content', []):
                if block.get('type') == 'text':
                    content += block.get('text', '')

            if not content:
                raise Exception("Empty response from AWS Bedrock service")

            return content

        except ClientError as e:
            logger.error(f"AWS Bedrock client error: {str(e)}")
            raise Exception(f"AWS Bedrock service error: {e.response['Error']['Code']}")
        except Exception as e:
            logger.error(f"Failed to generate response from AWS Bedrock: {str(e)}")
            raise Exception(f"Failed to generate response: {str(e)}")

    async def health_check(self) -> bool:
        """Check if AWS Bedrock service is accessible."""
        try:
            # Try a minimal request to check service accessibility
            body = {
                "anthropic_version": "bedrock-2023-05-31",
                "max_tokens": 1,
                "messages": [{"role": "user", "content": "Hello"}],
            }

            response = self.client.invoke_model(
                modelId=self.model_id,
                body=json.dumps(body),
                contentType="application/json",
                accept="application/json",
            )
            return True
        except Exception:
            return False
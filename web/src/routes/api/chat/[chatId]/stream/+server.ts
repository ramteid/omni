import { json } from '@sveltejs/kit'
import { env } from '$env/dynamic/private'
import type { RequestHandler } from './$types.js'

async function triggerTitleGeneration(chatId: string, logger: any): Promise<void> {
    try {
        const response = await fetch(`${env.AI_SERVICE_URL}/chat/${chatId}/generate_title`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
        })

        if (response.ok) {
            const result = await response.json()
            logger.info('Title generation completed', {
                chatId,
                title: result.title,
                status: result.status,
            })
        } else {
            logger.warn('Title generation failed', undefined, { chatId, status: response.status })
        }
    } catch (error) {
        logger.warn('Error during title generation', error, { chatId })
    }
}

export const GET: RequestHandler = async ({ params, locals }) => {
    const logger = locals.logger.child('chat')

    const chatId = params.chatId
    if (!chatId) {
        logger.warn('Missing chatId parameter in stream request')
        return json({ error: 'chatId parameter is required' }, { status: 400 })
    }

    logger.debug('Sending GET request to AI service to receive the streaming response', { chatId })

    try {
        const response = await fetch(`${env.AI_SERVICE_URL}/chat/${chatId}/stream`)

        if (!response.ok) {
            logger.error('AI service error', undefined, {
                status: response.status,
                statusText: response.statusText,
                chatId,
            })
            return json(
                {
                    error: 'AI service unavailable',
                    details: `Status: ${response.status}`,
                },
                { status: 502 },
            )
        }

        logger.info('Chat stream started successfully', { chatId })

        // Create a transformed stream that triggers title generation after completion
        const reader = response.body?.getReader()

        if (!reader) {
            throw new Error('Response body is null')
        }

        const stream = new ReadableStream({
            async start(controller) {
                try {
                    while (true) {
                        const { done, value } = await reader.read()

                        if (done) {
                            // Stream completed - trigger title generation in background
                            logger.info('Stream completed, triggering title generation', { chatId })
                            triggerTitleGeneration(chatId, logger).catch((err) => {
                                logger.warn(
                                    `Failed to trigger title generation for chat ${chatId}`,
                                    err,
                                )
                            })
                            controller.close()
                            break
                        }

                        // Pass through the chunk to the client
                        controller.enqueue(value)
                    }
                } catch (error) {
                    logger.error('Error in stream processing', error, { chatId })
                    controller.error(error)
                }
            },
        })

        // Return the streaming response with SSE headers
        return new Response(stream, {
            status: 200,
            headers: {
                'Content-Type': 'text/event-stream',
                'Cache-Control': 'no-cache',
                Connection: 'keep-alive',
            },
        })
    } catch (error) {
        logger.error('Error calling AI service', error, { chatId })
        return json(
            {
                error: 'Failed to process request',
                details: error instanceof Error ? error.message : 'Unknown error',
            },
            { status: 500 },
        )
    }
}

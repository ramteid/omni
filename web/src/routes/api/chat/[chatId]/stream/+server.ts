import { json } from '@sveltejs/kit'
import { env } from '$env/dynamic/private'
import type { RequestHandler } from './$types.js'

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

        logger.info('Chat stream request completed successfully', { chatId })

        // Return the streaming response directly with SSE headers
        return new Response(response.body, {
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

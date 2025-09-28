import { json } from '@sveltejs/kit'
import type { RequestHandler } from './$types.js'
import { chatRepository } from '$lib/server/db/chats'

export const GET: RequestHandler = async ({ params, locals }) => {
    const logger = locals.logger.child('chat')

    const chatId = params.chatId
    if (!chatId) {
        logger.warn('Missing chatId parameter in chat details request')
        return json({ error: 'chatId parameter is required' }, { status: 400 })
    }

    logger.debug('Fetching chat details', { chatId })

    try {
        const chat = await chatRepository.get(chatId)

        if (!chat) {
            logger.warn('Chat not found', { chatId })
            return json({ error: 'Chat not found' }, { status: 404 })
        }

        logger.info('Chat details retrieved successfully', { chatId })

        // Convert to match AI service response format
        const chatDetails = {
            id: chat.id,
            user_id: chat.userId,
            title: chat.title,
            created_at: chat.createdAt,
            updated_at: chat.updatedAt,
        }

        return json(chatDetails, { status: 200 })
    } catch (error) {
        logger.error('Error fetching chat details', error, { chatId })
        return json(
            {
                error: 'Failed to fetch chat details',
                details: error instanceof Error ? error.message : 'Unknown error',
            },
            { status: 500 },
        )
    }
}

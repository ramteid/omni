import { chatRepository, chatMessageRepository } from '$lib/server/db/chats.js'
import { error } from '@sveltejs/kit'

export const load = async ({ params, locals }) => {
    const chat = await chatRepository.get(params.chatId)
    if (!chat) {
        // throw 404
        error(404, 'Chat not found')
    }

    const messages = await chatMessageRepository.getByChatId(chat.id)

    return {
        user: locals.user!,
        chat,
        messages,
    }
}

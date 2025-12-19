import { error, fail } from '@sveltejs/kit'
import type { PageServerLoad, Actions } from './$types'
import { requireAdmin } from '$lib/server/authHelpers'
import {
    getEmbeddingConfig,
    upsertEmbeddingConfig,
    type EmbeddingProvider,
} from '$lib/server/db/embedding-config'

export const load: PageServerLoad = async ({ locals }) => {
    requireAdmin(locals)

    const config = await getEmbeddingConfig()

    return {
        config: config
            ? {
                  provider: config.provider,
                  // Jina fields
                  jinaModel: config.jinaModel,
                  jinaApiUrl: config.jinaApiUrl,
                  // Bedrock fields
                  bedrockModelId: config.bedrockModelId,
                  // OpenAI fields
                  openaiModel: config.openaiModel,
                  openaiDimensions: config.openaiDimensions,
                  // Local fields
                  localBaseUrl: config.localBaseUrl,
                  localModel: config.localModel,
              }
            : null,
        hasJinaApiKey: config?.jinaApiKey ? true : false,
        hasOpenaiApiKey: config?.openaiApiKey ? true : false,
    }
}

export const actions: Actions = {
    save: async ({ request, locals }) => {
        requireAdmin(locals)

        const formData = await request.formData()

        const provider = formData.get('provider') as EmbeddingProvider
        // Jina fields
        const jinaApiKey = (formData.get('jinaApiKey') as string) || null
        const jinaModel = (formData.get('jinaModel') as string) || null
        const jinaApiUrl = (formData.get('jinaApiUrl') as string) || null
        // Bedrock fields
        const bedrockModelId = (formData.get('bedrockModelId') as string) || null
        // OpenAI fields
        const openaiApiKey = (formData.get('openaiApiKey') as string) || null
        const openaiModel = (formData.get('openaiModel') as string) || null
        const openaiDimensionsStr = formData.get('openaiDimensions') as string
        const openaiDimensions = openaiDimensionsStr ? parseInt(openaiDimensionsStr, 10) : null
        // Local fields
        const localBaseUrl = (formData.get('localBaseUrl') as string) || null
        const localModel = (formData.get('localModel') as string) || null

        // Validation
        if (!provider) {
            return fail(400, {
                error: 'Provider is required',
                provider,
            })
        }

        if (!['local', 'jina', 'openai', 'bedrock'].includes(provider)) {
            return fail(400, {
                error: 'Invalid provider',
                provider,
            })
        }

        // Provider-specific validation
        if (provider === 'local') {
            if (!localBaseUrl) {
                return fail(400, {
                    error: 'Base URL is required for Local provider',
                    provider,
                })
            }
            if (!localModel) {
                return fail(400, {
                    error: 'Model is required for Local provider',
                    provider,
                })
            }
        }

        if (provider === 'jina' && !jinaModel) {
            return fail(400, {
                error: 'Model is required for Jina provider',
                provider,
            })
        }

        if (provider === 'openai' && !openaiModel) {
            return fail(400, {
                error: 'Model is required for OpenAI provider',
                provider,
            })
        }

        if (provider === 'bedrock' && !bedrockModelId) {
            return fail(400, {
                error: 'Model ID is required for Bedrock provider',
                provider,
            })
        }

        try {
            // Get existing config to preserve API key if not provided (only for current provider)
            const existingConfig = await getEmbeddingConfig()

            // Only keep fields for the selected provider, clear all others
            await upsertEmbeddingConfig({
                provider,
                // Jina fields - only set if jina is selected
                jinaApiKey:
                    provider === 'jina' ? jinaApiKey || existingConfig?.jinaApiKey || null : null,
                jinaModel: provider === 'jina' ? jinaModel : null,
                jinaApiUrl: provider === 'jina' ? jinaApiUrl : null,
                // Bedrock fields - only set if bedrock is selected
                bedrockModelId: provider === 'bedrock' ? bedrockModelId : null,
                // OpenAI fields - only set if openai is selected
                openaiApiKey:
                    provider === 'openai'
                        ? openaiApiKey || existingConfig?.openaiApiKey || null
                        : null,
                openaiModel: provider === 'openai' ? openaiModel : null,
                openaiDimensions: provider === 'openai' ? openaiDimensions : null,
                // Local fields - only set if local is selected
                localBaseUrl: provider === 'local' ? localBaseUrl : null,
                localModel: provider === 'local' ? localModel : null,
            })

            return {
                success: true,
                message: 'Embedding configuration saved successfully',
            }
        } catch (err) {
            console.error('Failed to save embedding configuration:', err)
            return fail(500, {
                error: 'Failed to save embedding configuration',
            })
        }
    },
}

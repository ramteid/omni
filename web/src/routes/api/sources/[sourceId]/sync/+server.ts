import { json, error } from '@sveltejs/kit'
import type { RequestHandler } from './$types'
import { getConfig } from '$lib/server/config'
import { db } from '$lib/server/db'

export const POST: RequestHandler = async ({ params, fetch }) => {
    const { sourceId } = params

    if (!sourceId) {
        throw error(400, 'Source ID is required')
    }

    try {
        // Get the source from database to determine the connector type
        const source = await db
            .selectFrom('sources')
            .select(['id', 'source_type'])
            .where('id', '=', sourceId)
            .executeTakeFirst()

        if (!source) {
            throw error(404, 'Source not found')
        }

        const config = getConfig()

        // Determine which connector URL to use based on source type
        let connectorUrl: string
        switch (source.source_type) {
            case 'google_drive':
            case 'gmail':
                connectorUrl = config.services.googleConnectorUrl
                break
            case 'slack':
                connectorUrl = config.services.slackConnectorUrl
                break
            case 'confluence':
            case 'jira':
                connectorUrl = config.services.atlassianConnectorUrl
                break
            default:
                throw error(400, `Unsupported source type: ${source.source_type}`)
        }

        // Call the connector's sync endpoint
        const syncResponse = await fetch(`${connectorUrl}/sync/${sourceId}`, {
            method: 'POST',
        })

        if (!syncResponse.ok) {
            const errorText = await syncResponse.text()
            console.error(`Sync failed for source ${sourceId}:`, errorText)
            throw error(500, 'Failed to trigger sync')
        }

        const syncResult = await syncResponse.json()

        return json({
            success: true,
            message: 'Sync triggered successfully',
            sourceId,
            result: syncResult,
        })
    } catch (err) {
        console.error('Error triggering sync:', err)

        if (err && typeof err === 'object' && 'status' in err) {
            throw err // Re-throw SvelteKit errors
        }

        throw error(500, 'Internal server error')
    }
}

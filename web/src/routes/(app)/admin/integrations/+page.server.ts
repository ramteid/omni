import { requireAdmin } from '$lib/server/authHelpers'
import { db } from '$lib/server/db'
import { sources, connectorEventsQueue } from '$lib/server/db/schema'
import { sql, eq } from 'drizzle-orm'
import type { PageServerLoad } from './$types'

export const load: PageServerLoad = async ({ locals }) => {
    requireAdmin(locals)

    // Get all organization-level connected sources
    const connectedSources = await db.select().from(sources)

    // Get initial indexing status for each source
    const indexingStatus = await db
        .select({
            sourceId: connectorEventsQueue.sourceId,
            status: connectorEventsQueue.status,
            count: sql<number>`count(*)::int`,
        })
        .from(connectorEventsQueue)
        .groupBy(connectorEventsQueue.sourceId, connectorEventsQueue.status)

    // Transform indexing status into a more usable format
    const statusBySource = indexingStatus.reduce(
        (acc, item) => {
            if (!acc[item.sourceId]) {
                acc[item.sourceId] = {}
            }
            acc[item.sourceId][item.status] = item.count
            return acc
        },
        {} as Record<string, Record<string, number>>,
    )

    return {
        connectedSources,
        indexingStatus: statusBySource,
        availableIntegrations: [
            {
                id: 'google',
                name: 'Google Workspace',
                description: 'Connect to Google Drive, Docs, Gmail, and more',
                connected: connectedSources.some(
                    (source) =>
                        source.sourceType === 'google_drive' || source.sourceType === 'gmail',
                ),
                connectUrl: '/api/oauth/google/connect',
            },
            {
                id: 'slack',
                name: 'Slack',
                description: 'Connect to Slack messages and files',
                connected: connectedSources.some((source) => source.sourceType === 'slack'),
                connectUrl: '/api/oauth/slack/connect',
            },
            {
                id: 'atlassian',
                name: 'Atlassian',
                description: 'Connect to Confluence and Jira',
                connected: connectedSources.some(
                    (source) => source.sourceType === 'confluence' || source.sourceType === 'jira',
                ),
                connectUrl: '/api/oauth/atlassian/connect',
            },
        ],
    }
}

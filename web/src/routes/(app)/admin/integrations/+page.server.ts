import { requireAdmin } from '$lib/server/authHelpers'
import { db } from '$lib/server/db'
import { sources, syncRuns, documents } from '$lib/server/db/schema'
import { sql, eq, desc } from 'drizzle-orm'
import type { PageServerLoad } from './$types'

export const load: PageServerLoad = async ({ locals }) => {
    requireAdmin(locals)

    // Get all organization-level connected sources
    const connectedSources = await db.select().from(sources)

    // Get latest 10 sync runs
    const latestSyncRuns = await db
        .select({
            id: syncRuns.id,
            sourceId: syncRuns.sourceId,
            sourceName: sources.name,
            sourceType: sources.sourceType,
            syncType: syncRuns.syncType,
            status: syncRuns.status,
            documentsProcessed: syncRuns.documentsProcessed,
            documentsUpdated: syncRuns.documentsUpdated,
            startedAt: syncRuns.startedAt,
            completedAt: syncRuns.completedAt,
            errorMessage: syncRuns.errorMessage,
        })
        .from(syncRuns)
        .leftJoin(sources, eq(syncRuns.sourceId, sources.id))
        .orderBy(desc(syncRuns.startedAt))
        .limit(10)

    // Get actual document counts
    const documentsBySource = await db
        .select({
            sourceId: documents.sourceId,
            count: sql<number>`COUNT(*)::int`,
        })
        .from(documents)
        .groupBy(documents.sourceId)

    const totalDocumentsIndexed = documentsBySource
        .map((r) => r.count)
        .reduce((a, v) => (a += v), 0)

    const documentCountBySource = documentsBySource.reduce(
        (acc, item) => {
            acc[item.sourceId] = item.count
            return acc
        },
        {} as Record<string, number>,
    )

    return {
        connectedSources,
        latestSyncRuns,
        documentStats: {
            totalDocumentsIndexed,
            documentsBySource: documentCountBySource,
        },
        availableIntegrations: [
            {
                id: 'google',
                name: 'Google Workspace',
                description:
                    'Connect to Google Drive, Docs, Gmail, and more using a service account',
                connected: connectedSources.some(
                    (source) =>
                        source.sourceType === 'google_drive' || source.sourceType === 'gmail',
                ),
                authType: 'service_account',
            },
            {
                id: 'slack',
                name: 'Slack',
                description: 'Connect to Slack messages and files using a bot token',
                connected: connectedSources.some((source) => source.sourceType === 'slack'),
                authType: 'bot_token',
            },
            {
                id: 'atlassian',
                name: 'Atlassian',
                description: 'Connect to Confluence and Jira using an API token',
                connected: connectedSources.some(
                    (source) => source.sourceType === 'confluence' || source.sourceType === 'jira',
                ),
                authType: 'api_token',
            },
        ],
    }
}

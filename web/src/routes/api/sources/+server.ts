import { json, error } from '@sveltejs/kit'
import type { RequestHandler } from './$types'
import { db } from '$lib/server/db'
import { sources, oauthCredentials } from '$lib/server/db/schema'
import { eq, inArray } from 'drizzle-orm'

export const GET: RequestHandler = async ({ locals }) => {
    if (!locals.user) {
        throw error(401, 'Unauthorized')
    }

    const userSources = await db.query.sources.findMany({
        where: eq(sources.createdBy, locals.user.id),
    })

    // Get OAuth credentials for all sources
    const sourceIds = userSources.map((s) => s.id)
    const credentials =
        sourceIds.length > 0
            ? await db.query.oauthCredentials.findMany({
                  where: inArray(oauthCredentials.sourceId, sourceIds),
              })
            : []

    // Create a map of source ID to whether it has credentials
    const credentialsMap = new Map(credentials.map((c) => [c.sourceId, true]))

    const sanitizedSources = userSources.map((source) => ({
        id: source.id,
        name: source.name,
        sourceType: source.sourceType,
        config: source.config,
        syncStatus: source.syncStatus,
        isActive: source.isActive,
        lastSyncAt: source.lastSyncAt,
        syncError: source.syncError,
        createdAt: source.createdAt,
        updatedAt: source.updatedAt,
        isConnected: credentialsMap.has(source.id),
    }))

    return json(sanitizedSources)
}

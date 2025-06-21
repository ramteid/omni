import type { PageServerLoad } from './$types.js'
import { db } from '$lib/server/db/index.js'
import { sources, documents } from '$lib/server/db/schema.js'
import { eq, sql } from 'drizzle-orm'

export const load: PageServerLoad = async ({ locals }) => {
    // Get connected sources count
    const connectedSources = await db.select().from(sources).where(eq(sources.isActive, true))
    const connectedSourcesCount = connectedSources.length

    // Get total indexed documents count
    const documentsBySource = await db
        .select({
            count: sql<number>`COUNT(*)::int`,
        })
        .from(documents)

    const totalDocumentsIndexed = documentsBySource.length > 0 ? documentsBySource[0].count : 0

    return {
        user: locals.user!,
        stats: {
            connectedSources: connectedSourcesCount,
            indexedDocuments: totalDocumentsIndexed,
        },
    }
}

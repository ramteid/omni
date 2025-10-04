import { db } from '$lib/server/db'
import { documents } from '$lib/server/db/schema'
import { sql } from 'drizzle-orm'

export class DocumentsRepository {
    async getCountsBySource() {
        return await db
            .select({
                sourceId: documents.sourceId,
                count: sql<number>`COUNT(*)::int`,
            })
            .from(documents)
            .groupBy(documents.sourceId)
    }
}

export const documentsRepository = new DocumentsRepository()

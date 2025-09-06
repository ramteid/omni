import { eq, inArray } from 'drizzle-orm'
import { db } from './index'
import { sources, type Source } from './schema'

export type UserFilterMode = 'all' | 'whitelist' | 'blacklist'

export interface SourceUpdateData {
    isActive?: boolean
    userFilterMode?: UserFilterMode
    userWhitelist?: string[]
    userBlacklist?: string[]
    updatedAt?: Date
}

export async function getGoogleSources(): Promise<Source[]> {
    return await db
        .select()
        .from(sources)
        .where(inArray(sources.sourceType, ['google_drive', 'gmail']))
}

export async function getActiveGoogleSources(): Promise<Source[]> {
    return await db
        .select()
        .from(sources)
        .where(inArray(sources.sourceType, ['google_drive', 'gmail']))
        .where(eq(sources.isActive, true))
}

export async function updateGoogleSources(
    driveEnabled: boolean,
    gmailEnabled: boolean,
    driveSettings: {
        userFilterMode: UserFilterMode
        userWhitelist: string[] | null
        userBlacklist: string[] | null
    },
    gmailSettings: {
        userFilterMode: UserFilterMode
        userWhitelist: string[] | null
        userBlacklist: string[] | null
    },
): Promise<void> {
    await db.transaction(async (tx) => {
        // Get existing Google sources
        const googleSources = await tx
            .select()
            .from(sources)
            .where(inArray(sources.sourceType, ['google_drive', 'gmail']))

        const driveSource = googleSources.find((s) => s.sourceType === 'google_drive')
        const gmailSource = googleSources.find((s) => s.sourceType === 'gmail')

        // Update Google Drive source
        if (driveSource) {
            await tx
                .update(sources)
                .set({
                    isActive: driveEnabled,
                    userFilterMode: driveEnabled
                        ? driveSettings.userFilterMode
                        : driveSource.userFilterMode,
                    userWhitelist: driveEnabled
                        ? driveSettings.userWhitelist
                        : driveSource.userWhitelist,
                    userBlacklist: driveEnabled
                        ? driveSettings.userBlacklist
                        : driveSource.userBlacklist,
                    updatedAt: new Date(),
                })
                .where(eq(sources.id, driveSource.id))
        }

        // Update Gmail source
        if (gmailSource) {
            await tx
                .update(sources)
                .set({
                    isActive: gmailEnabled,
                    userFilterMode: gmailEnabled
                        ? gmailSettings.userFilterMode
                        : gmailSource.userFilterMode,
                    userWhitelist: gmailEnabled
                        ? gmailSettings.userWhitelist
                        : gmailSource.userWhitelist,
                    userBlacklist: gmailEnabled
                        ? gmailSettings.userBlacklist
                        : gmailSource.userBlacklist,
                    updatedAt: new Date(),
                })
                .where(eq(sources.id, gmailSource.id))
        }
    })
}

export async function updateSource(sourceId: string, data: SourceUpdateData): Promise<void> {
    const updateData: any = { ...data }

    if (data.userWhitelist) {
        updateData.userWhitelist = data.userWhitelist
    }
    if (data.userBlacklist) {
        updateData.userBlacklist = data.userBlacklist
    }

    await db.update(sources).set(updateData).where(eq(sources.id, sourceId))
}

export async function getSourceById(sourceId: string): Promise<Source | undefined> {
    const result = await db.select().from(sources).where(eq(sources.id, sourceId)).limit(1)

    return result[0]
}

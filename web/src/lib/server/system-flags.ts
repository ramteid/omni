import { getRedisClient } from './redis'

const SYSTEM_FLAGS_KEY = 'system:flags'

export class SystemFlags {
    private static memoryCache: Map<string, boolean> = new Map()

    /**
     * Check if the system has been initialized (first admin created)
     */
    static async isInitialized(): Promise<boolean> {
        // Check memory cache first
        if (this.memoryCache.has('initialized')) {
            return this.memoryCache.get('initialized')!
        }

        // Check Redis
        const redis = await getRedisClient()
        const value = await redis.hGet(SYSTEM_FLAGS_KEY, 'initialized')
        const initialized = value === 'true'

        // Cache in memory
        this.memoryCache.set('initialized', initialized)

        return initialized
    }

    /**
     * Mark system as initialized (called after first admin account creation)
     */
    static async markAsInitialized(): Promise<void> {
        const redis = await getRedisClient()
        await redis.hSet(SYSTEM_FLAGS_KEY, 'initialized', 'true')
        this.memoryCache.set('initialized', true)
    }

    /**
     * Reset initialization flag (useful for testing)
     */
    static async resetInitialization(): Promise<void> {
        const redis = await getRedisClient()
        await redis.hDel(SYSTEM_FLAGS_KEY, 'initialized')
        this.memoryCache.delete('initialized')
    }

    /**
     * Clear memory cache (useful if Redis was updated externally)
     */
    static clearCache(): void {
        this.memoryCache.clear()
    }
}

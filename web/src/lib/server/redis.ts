import { createClient } from 'redis'
import { getConfig } from './config'
import { createLogger } from './logger.js'

const logger = createLogger('redis')

const config = getConfig()

let redisClient: ReturnType<typeof createClient> | null = null

export async function getRedisClient() {
    if (!redisClient) {
        redisClient = createClient({
            url: config.redis.url,
        })

        redisClient.on('error', (err) => {
            logger.error('Redis connection error', err)
        })

        redisClient.on('connect', () => {
            logger.info('Connected to Redis')
        })

        redisClient.on('ready', () => {
            logger.info('Redis client ready')
        })

        redisClient.on('end', () => {
            logger.info('Redis connection ended')
        })

        await redisClient.connect()
    }

    return redisClient
}

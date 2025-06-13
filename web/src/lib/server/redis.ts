import { createClient } from 'redis';
import { redis } from './config';

export async function createRedisClient() {
	const client = createClient({
		url: redis.url
	});

	await client.connect();
	return client;
}
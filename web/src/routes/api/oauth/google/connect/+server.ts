import { redirect, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { createRedisClient } from '$lib/server/redis';
import crypto from 'crypto';

const GOOGLE_AUTH_URL = 'https://accounts.google.com/o/oauth2/v2/auth';
const SCOPES = [
	'https://www.googleapis.com/auth/drive.readonly',
	'https://www.googleapis.com/auth/userinfo.email',
	'https://www.googleapis.com/auth/userinfo.profile'
];

export const GET: RequestHandler = async ({ url, locals }) => {
	if (!locals.user) {
		throw error(401, 'Unauthorized');
	}

	const clientId = process.env.GOOGLE_CLIENT_ID;
	const redirectUri = process.env.GOOGLE_REDIRECT_URI || `${url.origin}/api/oauth/google/callback`;

	if (!clientId) {
		throw error(500, 'Google OAuth not configured');
	}

	const state = crypto.randomBytes(32).toString('hex');
	
	const redis = await createRedisClient();
	await redis.setex(`oauth:state:${state}`, 300, JSON.stringify({
		userId: locals.user.id,
		timestamp: Date.now()
	}));
	await redis.quit();

	const params = new URLSearchParams({
		client_id: clientId,
		redirect_uri: redirectUri,
		response_type: 'code',
		scope: SCOPES.join(' '),
		state,
		access_type: 'offline',
		prompt: 'consent'
	});

	throw redirect(302, `${GOOGLE_AUTH_URL}?${params}`);
};
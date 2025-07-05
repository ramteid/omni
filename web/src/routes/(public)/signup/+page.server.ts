import { fail, redirect } from '@sveltejs/kit'
import { hash } from '@node-rs/argon2'
import { eq, count } from 'drizzle-orm'
import { db } from '$lib/server/db/index.js'
import { user } from '$lib/server/db/schema.js'
import { ulid } from 'ulid'
import { generateSessionToken, createSession, setSessionTokenCookie } from '$lib/server/auth.js'
import { DomainService } from '$lib/server/domains.js'
import type { Actions, PageServerLoad } from './$types.js'

export const load: PageServerLoad = async ({ locals }) => {
    if (locals.user) {
        throw redirect(302, '/')
    }
    return {}
}

export const actions: Actions = {
    default: async ({ request, cookies }) => {
        const formData = await request.formData()
        const email = formData.get('email') as string
        const password = formData.get('password') as string
        const confirmPassword = formData.get('confirmPassword') as string

        // Basic validation
        if (!email || !password || !confirmPassword) {
            return fail(400, {
                error: 'All fields are required.',
                email,
            })
        }

        if (
            typeof email !== 'string' ||
            typeof password !== 'string' ||
            typeof confirmPassword !== 'string'
        ) {
            return fail(400, {
                error: 'Invalid form data.',
                email,
            })
        }

        // Email validation
        const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/
        if (!emailRegex.test(email)) {
            return fail(400, {
                error: 'Please enter a valid email address.',
                email,
            })
        }

        // Password validation
        if (password.length < 8) {
            return fail(400, {
                error: 'Password must be at least 8 characters long.',
                email,
            })
        }

        if (password !== confirmPassword) {
            return fail(400, {
                error: 'Passwords do not match.',
                email,
            })
        }

        try {
            // Check if email already exists
            const [existingEmail] = await db
                .select()
                .from(user)
                .where(eq(user.email, email.toLowerCase()))

            if (existingEmail) {
                return fail(400, {
                    error: 'An account with this email already exists.',
                    email,
                })
            }

            // Check if this is the first user
            const [userCount] = await db.select({ count: count() }).from(user)
            const isFirstUser = userCount.count === 0

            // Hash password
            const passwordHash = await hash(password, {
                memoryCost: 65536,
                timeCost: 3,
                outputLen: 32,
                parallelism: 1,
            })

            // Extract domain from email
            const domain = DomainService.extractDomainFromEmail(email.toLowerCase())

            // Create user
            const newUserId = ulid()
            await db.insert(user).values({
                id: newUserId,
                email: email.toLowerCase(),
                passwordHash,
                role: isFirstUser ? 'admin' : 'user',
                authMethod: 'password',
                domain,
                isActive: true,
                createdAt: new Date(),
            })

            // If this is the first user (admin), auto-approve their domain
            if (isFirstUser && domain) {
                await DomainService.autoApproveDomainForAdmin(newUserId)
            }

            // Create session and log the user in
            const sessionToken = generateSessionToken()
            const session = await createSession(sessionToken, newUserId)
            setSessionTokenCookie(cookies, sessionToken, session.expiresAt)
        } catch (error) {
            console.error('Registration error:', error)
            return fail(500, {
                error: 'An unexpected error occurred. Please try again.',
                email,
            })
        }

        // Redirect to home page after successful signup and login
        throw redirect(302, '/')
    },
}

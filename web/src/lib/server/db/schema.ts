import { pgTable, serial, integer, text, timestamp, jsonb, varchar } from 'drizzle-orm/pg-core';

export const user = pgTable('user', {
	id: text('id').primaryKey(),
	age: integer('age'),
	username: text('username').notNull().unique(),
	passwordHash: text('password_hash').notNull()
});

export const session = pgTable('session', {
	id: text('id').primaryKey(),
	userId: text('user_id')
		.notNull()
		.references(() => user.id),
	expiresAt: timestamp('expires_at', { withTimezone: true, mode: 'date' }).notNull()
});

export const sources = pgTable('sources', {
	id: text('id').primaryKey(),
	name: varchar('name', { length: 255 }).notNull(),
	sourceType: varchar('source_type', { length: 50 }).notNull(),
	config: jsonb('config').notNull().default({}),
	oauthCredentials: jsonb('oauth_credentials'),
	isActive: boolean('is_active').notNull().default(true),
	lastSyncAt: timestamp('last_sync_at', { withTimezone: true, mode: 'date' }),
	syncStatus: varchar('sync_status', { length: 50 }).default('pending'),
	syncError: text('sync_error'),
	createdAt: timestamp('created_at', { withTimezone: true, mode: 'date' }).notNull().defaultNow(),
	updatedAt: timestamp('updated_at', { withTimezone: true, mode: 'date' }).notNull().defaultNow(),
	createdBy: text('created_by')
		.notNull()
		.references(() => user.id)
});

export type Session = typeof session.$inferSelect;

export type User = typeof user.$inferSelect;

export type Source = typeof sources.$inferSelect;

import { defineConfig } from 'drizzle-kit'
import { constructDatabaseUrl } from './src/lib/server/config.js'

const databaseUrl = constructDatabaseUrl()

export default defineConfig({
    schema: './src/lib/server/db/schema.ts',
    dialect: 'postgresql',
    dbCredentials: { url: databaseUrl },
    verbose: true,
    strict: true,
})

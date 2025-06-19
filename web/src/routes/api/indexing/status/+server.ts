import { db } from '$lib/server/db/index.js'
import { syncRuns, sources } from '$lib/server/db/schema.js'
import { sql, eq, desc } from 'drizzle-orm'
import type { RequestHandler } from './$types.js'
import { Client } from 'pg'
import { DATABASE_URL } from '$env/static/private'

export const GET: RequestHandler = async ({ url }) => {
    const stream = new ReadableStream({
        async start(controller) {
            const encoder = new TextEncoder()
            let isClosed = false
            let pgClient: Client | null = null

            // Function to send data to client
            const sendData = (data: any) => {
                if (isClosed) return

                try {
                    const message = `data: ${JSON.stringify(data)}\n\n`
                    controller.enqueue(encoder.encode(message))
                } catch (error) {
                    console.error('Error sending SSE data:', error)
                    isClosed = true
                }
            }

            // Function to fetch and send status updates
            const fetchStatus = async () => {
                if (isClosed) return

                try {
                    // Get currently running sync runs (these are actively being updated by indexer)
                    const runningSyncRuns = await db
                        .select({
                            id: syncRuns.id,
                            sourceId: syncRuns.sourceId,
                            sourceName: sources.name,
                            sourceType: sources.sourceType,
                            syncType: syncRuns.syncType,
                            documentsProcessed: syncRuns.documentsProcessed,
                            documentsUpdated: syncRuns.documentsUpdated,
                            startedAt: syncRuns.startedAt,
                            errorMessage: syncRuns.errorMessage,
                        })
                        .from(syncRuns)
                        .leftJoin(sources, eq(syncRuns.sourceId, sources.id))
                        .where(eq(syncRuns.status, 'running'))
                        .orderBy(desc(syncRuns.startedAt))

                    // Get recently completed or failed sync runs for context
                    const recentCompletedRuns = await db
                        .select({
                            id: syncRuns.id,
                            sourceId: syncRuns.sourceId,
                            sourceName: sources.name,
                            sourceType: sources.sourceType,  
                            syncType: syncRuns.syncType,
                            status: syncRuns.status,
                            documentsProcessed: syncRuns.documentsProcessed,
                            documentsUpdated: syncRuns.documentsUpdated,
                            startedAt: syncRuns.startedAt,
                            completedAt: syncRuns.completedAt,
                            errorMessage: syncRuns.errorMessage,
                        })
                        .from(syncRuns)
                        .leftJoin(sources, eq(syncRuns.sourceId, sources.id))
                        .where(sql`${syncRuns.status} IN ('completed', 'failed')`)
                        .orderBy(desc(syncRuns.completedAt))
                        .limit(10)

                    // Get overall status counts
                    const overallStatus = await db
                        .select({
                            status: syncRuns.status,
                            count: sql<number>`count(*)::int`,
                        })
                        .from(syncRuns)
                        .groupBy(syncRuns.status)

                    const overallStatusMap = overallStatus.reduce(
                        (acc, item) => {
                            acc[item.status] = item.count
                            return acc
                        },
                        {} as Record<string, number>,
                    )

                    const statusData = {
                        timestamp: Date.now(),
                        overall: {
                            running: overallStatusMap.running || 0,
                            completed: overallStatusMap.completed || 0,
                            failed: overallStatusMap.failed || 0,
                        },
                        runningSyncs: runningSyncRuns,
                        recentActivity: recentCompletedRuns,
                    }

                    sendData(statusData)
                } catch (error) {
                    console.error('Error fetching indexing status:', error)
                    if (!isClosed) {
                        sendData({ error: 'Failed to fetch status' })
                    }
                }
            }

            // Setup PostgreSQL LISTEN/NOTIFY for real-time updates
            const setupNotifications = async () => {
                try {
                    pgClient = new Client({
                        connectionString: DATABASE_URL,
                    })
                    await pgClient.connect()
                    
                    // Listen for sync_runs updates
                    await pgClient.query('LISTEN sync_run_update')
                    
                    pgClient.on('notification', async (msg) => {
                        if (msg.channel === 'sync_run_update' && !isClosed) {
                            // Fetch and send updated status when we receive notification
                            await fetchStatus()
                        }
                    })
                } catch (error) {
                    console.error('Error setting up PostgreSQL notifications:', error)
                    // Fall back to polling if LISTEN/NOTIFY fails
                    const interval = setInterval(fetchStatus, 5000)
                    return () => clearInterval(interval)
                }
            }

            // Send initial data
            await fetchStatus()

            // Setup real-time notifications
            const cleanupNotifications = await setupNotifications()

            // Cleanup on connection close
            return () => {
                isClosed = true
                if (pgClient) {
                    pgClient.end().catch(console.error)
                }
                if (cleanupNotifications) {
                    cleanupNotifications()
                }
                try {
                    controller.close()
                } catch (error) {
                    // Controller might already be closed
                }
            }
        },
    })

    return new Response(stream, {
        headers: {
            'Content-Type': 'text/event-stream',
            'Cache-Control': 'no-cache',
            Connection: 'keep-alive',
            'Access-Control-Allow-Origin': '*',
            'Access-Control-Allow-Headers': 'Cache-Control',
        },
    })
}

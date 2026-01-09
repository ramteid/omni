import express, { type Express, type Request, type Response } from 'express';
import pg from 'pg';

import { SdkClient } from './client.js';
import type { Connector } from './connector.js';
import { SyncContext } from './context.js';
import {
  SyncRequestSchema,
  CancelRequestSchema,
  ActionRequestSchema,
  createSyncResponseStarted,
  createSyncResponseError,
  createActionResponseFailure,
} from './models.js';

const { Pool } = pg;

interface SourceData {
  config: Record<string, unknown>;
  credentials: Record<string, unknown>;
  state: Record<string, unknown> | null;
}

async function fetchSourceData(sourceId: string): Promise<SourceData> {
  let connectionString = process.env.DATABASE_URL;
  if (!connectionString) {
    const host = process.env.DATABASE_HOST ?? 'localhost';
    const username = process.env.DATABASE_USERNAME ?? 'postgres';
    const password = process.env.DATABASE_PASSWORD ?? 'postgres';
    const dbname = process.env.DATABASE_NAME ?? 'omni';
    const port = process.env.DATABASE_PORT ?? '5432';
    connectionString = `postgresql://${username}:${password}@${host}:${port}/${dbname}`;
  }

  const pool = new Pool({ connectionString });
  try {
    const sourceResult = await pool.query(
      'SELECT config, connector_state FROM sources WHERE id = $1',
      [sourceId]
    );

    if (sourceResult.rows.length === 0) {
      throw new Error(`Source not found: ${sourceId}`);
    }

    const source = sourceResult.rows[0];

    const credsResult = await pool.query(
      'SELECT credentials FROM service_credentials WHERE source_id = $1',
      [sourceId]
    );

    const config = source.config ? JSON.parse(source.config) : {};
    const state = source.connector_state
      ? JSON.parse(source.connector_state)
      : null;
    const credentials =
      credsResult.rows.length > 0 && credsResult.rows[0].credentials
        ? JSON.parse(credsResult.rows[0].credentials)
        : {};

    return { config, credentials, state };
  } finally {
    await pool.end();
  }
}

export function createServer(connector: Connector): Express {
  const app = express();
  app.use(express.json());

  const activeSyncs = new Map<string, SyncContext>();
  let sdkClient: SdkClient | null = null;

  function getSdkClient(): SdkClient {
    if (sdkClient === null) {
      sdkClient = SdkClient.fromEnv();
    }
    return sdkClient;
  }

  app.get('/health', (_req: Request, res: Response) => {
    res.json({ status: 'healthy', service: connector.name });
  });

  app.get('/manifest', (_req: Request, res: Response) => {
    res.json(connector.getManifest());
  });

  app.post('/sync', async (req: Request, res: Response) => {
    const parseResult = SyncRequestSchema.safeParse(req.body);
    if (!parseResult.success) {
      res.status(400).json(createSyncResponseError('Invalid request body'));
      return;
    }

    const { sync_run_id: syncRunId, source_id: sourceId } = parseResult.data;

    console.log(`Sync triggered for source ${sourceId} (sync_run_id: ${syncRunId})`);

    if (activeSyncs.has(sourceId)) {
      res.status(409).json(
        createSyncResponseError('Sync already in progress for this source')
      );
      return;
    }

    let sourceData: SourceData;
    try {
      sourceData = await fetchSourceData(sourceId);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      if (message.includes('not found')) {
        res.status(404).json(createSyncResponseError(message));
      } else {
        console.error('Failed to fetch source data:', error);
        res.status(500).json(
          createSyncResponseError(`Failed to fetch source data: ${message}`)
        );
      }
      return;
    }

    const ctx = new SyncContext(
      getSdkClient(),
      syncRunId,
      sourceId,
      sourceData.state ?? undefined
    );
    activeSyncs.set(sourceId, ctx);

    const runSync = async (): Promise<void> => {
      try {
        await connector.sync(
          sourceData.config,
          sourceData.credentials,
          sourceData.state,
          ctx
        );
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        console.error(`Sync ${syncRunId} failed:`, error);
        try {
          await ctx.fail(message);
        } catch (failError) {
          console.error('Failed to report sync failure:', failError);
        }
      } finally {
        activeSyncs.delete(sourceId);
      }
    };

    runSync();

    res.status(200).json(createSyncResponseStarted());
  });

  app.post('/cancel', (req: Request, res: Response) => {
    const parseResult = CancelRequestSchema.safeParse(req.body);
    if (!parseResult.success) {
      res.status(400).json({ status: 'error', message: 'Invalid request body' });
      return;
    }

    const { sync_run_id: syncRunId } = parseResult.data;
    console.log(`Cancel requested for sync ${syncRunId}`);

    for (const [sourceId, ctx] of activeSyncs.entries()) {
      if (ctx.syncRunId === syncRunId) {
        ctx._setCancelled();
        connector.cancel(syncRunId);
        res.json({ status: 'cancelled' });
        return;
      }
    }

    res.json({ status: 'not_found' });
  });

  app.post('/action', async (req: Request, res: Response) => {
    const parseResult = ActionRequestSchema.safeParse(req.body);
    if (!parseResult.success) {
      res.status(400).json(createActionResponseFailure('Invalid request body'));
      return;
    }

    const { action, params, credentials } = parseResult.data;
    console.log(`Action requested: ${action}`);

    try {
      const response = await connector.executeAction(action, params, credentials);
      res.json(response);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      console.error(`Action ${action} failed:`, error);
      res.json(createActionResponseFailure(message));
    }
  });

  return app;
}

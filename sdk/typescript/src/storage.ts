import type { SdkClient } from './client.js';

export class ContentStorage {
  private readonly client: SdkClient;
  private readonly syncRunId: string;

  constructor(client: SdkClient, syncRunId: string) {
    this.client = client;
    this.syncRunId = syncRunId;
  }

  async save(content: string, contentType = 'text/plain'): Promise<string> {
    return this.client.storeContent(this.syncRunId, content, contentType);
  }

  async saveBinary(
    content: Buffer,
    contentType = 'application/octet-stream'
  ): Promise<string> {
    const encoded = content.toString('base64');
    return this.client.storeContent(this.syncRunId, encoded, contentType);
  }
}

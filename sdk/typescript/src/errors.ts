export class ConnectorError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'ConnectorError';
  }
}

export class SdkClientError extends ConnectorError {
  public readonly statusCode?: number;

  constructor(message: string, statusCode?: number) {
    super(message);
    this.name = 'SdkClientError';
    this.statusCode = statusCode;
  }
}

export class SyncCancelledError extends ConnectorError {
  constructor(message = 'Sync was cancelled') {
    super(message);
    this.name = 'SyncCancelledError';
  }
}

export class ConfigurationError extends ConnectorError {
  constructor(message: string) {
    super(message);
    this.name = 'ConfigurationError';
  }
}

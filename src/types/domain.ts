/// Types shared with the Rust backend (serde camelCase payloads).
export interface AppInfo {
  name: string;
  version: string;
  dataDir: string;
  dbPath: string;
  logDir: string;
  os: string;
  schemaVersion: number;
}

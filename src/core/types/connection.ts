/**
 * Shared types for SSH connection components (terminal view, dashboard).
 */

export interface ServerStatus {
  cpuUsage: number;
  memUsage: number;
  memTotal: number;
  memUsed: number;
  memAvail: number;
  swapUsage: number;
  swapTotal: number;
  swapUsed: number;
  diskUsage: number;
  diskTotal: number;
  diskUsed: number;
  diskAvail: number;
  netUp: number;
  netDown: number;
  latency: number;
  loadAvg: [number, number, number];
  uptime: string;
}

export interface UploadTask {
  id: string;
  fileName: string;
  remotePath?: string;
  status: 'pending' | 'uploading' | 'paused' | 'success' | 'error' | 'cancelled';
  progress: number;
  message: string;
  timestamp: number;
  error?: string;
  fileSize?: number;
  uploadedBytes?: number;
  startTime?: number;
  speed?: number;
  eta?: number;
}

/** Format a byte count into a human-readable string (e.g. "1.2 MB"). */
export function formatBytes(bytes: number): string {
  if (!bytes) return '0 B';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024)
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

/** Format a byte count as MiB/GiB (memory-style display). */
export function formatSizeMiB(bytes: number): string {
  if (!bytes) return '0 MiB';
  const mb = bytes / (1024 * 1024);
  if (mb < 1024) return `${mb.toFixed(1)} MiB`;
  return `${(mb / 1024).toFixed(1)} GiB`;
}

/** Format a transfer speed in bytes/second. */
export function formatSpeed(bytesPerSec: number): string {
  if (!bytesPerSec || bytesPerSec < 0.1) return '0 B/s';
  if (bytesPerSec < 1024) return `${bytesPerSec.toFixed(0)} B/s`;
  if (bytesPerSec < 1024 * 1024)
    return `${(bytesPerSec / 1024).toFixed(1)} KB/s`;
  return `${(bytesPerSec / (1024 * 1024)).toFixed(1)} MB/s`;
}

/** Format an ETA (seconds) as "Xm Ys" or "--:--" when unknown. */
export function formatETA(seconds: number): string {
  if (!seconds || seconds <= 0) return '--:--';
  const minutes = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${minutes}m ${secs}s`;
}

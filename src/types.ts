export type DownloadStatus =
  | "active"
  | "waiting"
  | "paused"
  | "complete"
  | "error"
  | "removed";

export interface Download {
  gid: string;
  uri: string;
  filename: string;
  dir: string;
  status: DownloadStatus;
  total_length: number;
  completed_length: number;
  download_speed: number;
  upload_speed: number;
  progress: number;
  eta_secs: number | null;
  error_message: string | null;
  created_at: number;
}

export interface GlobalStat {
  download_speed: number;
  upload_speed: number;
  num_active: number;
  num_waiting: number;
  num_stopped: number;
}

export interface Snapshot {
  downloads: Download[];
  global: GlobalStat;
  aria2_version: string | null;
  startup_error: string | null;
}

export type FilterKey =
  | "all"
  | "active"
  | "waiting"
  | "paused"
  | "complete"
  | "error";

export interface Settings {
  default_dir: string;
  default_split: number;
  max_concurrent_downloads: number;
  global_speed_limit: number;
  notify_on_complete: boolean;
  watch_clipboard: boolean;
}

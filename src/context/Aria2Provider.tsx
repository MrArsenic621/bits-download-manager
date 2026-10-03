import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import type { Download, Settings, Snapshot } from "../types";

export interface AddDownloadOptions {
  uri: string;
  dir?: string;
  out?: string;
  split?: number;
  referer?: string;
  userAgent?: string;
  cookie?: string;
  authUser?: string;
  authPass?: string;
}

interface Aria2ContextType {
  snapshot: Snapshot;
  connected: boolean;
  lastError: string | null;
  settings: Settings | null;
  addDownload: (opts: AddDownloadOptions) => Promise<string>;
  pause: (gid: string) => Promise<void>;
  resume: (gid: string) => Promise<void>;
  remove: (gid: string) => Promise<void>;
  del: (gid: string) => Promise<void>;
  pauseAll: () => Promise<void>;
  resumeAll: () => Promise<void>;
  clearFinished: () => Promise<void>;
  updateSettings: (settings: Settings) => Promise<void>;
  setSpeedLimit: (gid: string, limit: number) => Promise<void>;
}

const Aria2Context = createContext<Aria2ContextType | undefined>(undefined);

const EMPTY: Snapshot = {
  downloads: [],
  global: {
    download_speed: 0,
    upload_speed: 0,
    num_active: 0,
    num_waiting: 0,
    num_stopped: 0,
  },
  aria2_version: null,
  startup_error: null,
};

function isSnapshot(payload: unknown): payload is Snapshot {
  return (
    typeof payload === "object" &&
    payload !== null &&
    "downloads" in payload &&
    "global" in payload
  );
}

export const Aria2Provider: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => {
  const [snapshot, setSnapshot] = useState<Snapshot>(EMPTY);
  const [lastError, setLastError] = useState<string | null>(null);
  const [connected, setConnected] = useState(false);
  const [settings, setSettings] = useState<Settings | null>(null);
  const seenGids = useRef<Set<string>>(new Set());
  const prevStatuses = useRef<Map<string, string>>(new Map());
  const settingsRef = useRef<Settings | null>(null);
  settingsRef.current = settings;

  const notifyComplete = useCallback(async (d: Download) => {
    try {
      let granted = await isPermissionGranted();
      if (!granted) {
        granted = (await requestPermission()) === "granted";
      }
      if (granted) {
        sendNotification({
          title: "Download complete",
          body: d.filename || d.uri,
        });
      }
    } catch {
      // notifications unsupported; ignore
    }
  }, []);

  const apply = useCallback(
    (payload: unknown) => {
      if (isSnapshot(payload)) {
        setSnapshot(payload);
        setConnected(payload.startup_error == null);
        setLastError(payload.startup_error);
        payload.downloads.forEach((d) => {
          seenGids.current.add(d.gid);
          const prev = prevStatuses.current.get(d.gid);
          if (prev && prev !== "complete" && d.status === "complete") {
            if (settingsRef.current?.notify_on_complete) {
              void notifyComplete(d);
            }
          }
          prevStatuses.current.set(d.gid, d.status);
        });
      } else {
        setConnected(false);
      }
    },
    [notifyComplete],
  );

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;

    invoke("get_snapshot")
      .then((s) => {
        if (!cancelled) apply(s);
      })
      .catch((e) => setLastError(String(e)));

    invoke<Settings>("get_settings")
      .then((s) => {
        if (!cancelled) setSettings(s);
      })
      .catch((e) => setLastError(String(e)));

    listen<unknown>("downloads://update", (event) => {
      apply(event.payload);
    }).then((fn) => {
      unlisten = fn;
    });

    const interval = setInterval(() => {
      invoke("get_snapshot")
        .then((s) => {
          if (!cancelled) apply(s);
        })
        .catch(() => {});
    }, 3000);

    return () => {
      cancelled = true;
      unlisten?.();
      clearInterval(interval);
    };
  }, [apply]);

  const addDownload = useCallback(async (opts: AddDownloadOptions) => {
    const gid = await invoke<string>("add_download", {
      uri: opts.uri,
      dir: opts.dir ?? null,
      out: opts.out ?? null,
      split: opts.split ?? null,
      referer: opts.referer ?? null,
      userAgent: opts.userAgent ?? null,
      cookie: opts.cookie ?? null,
      authUser: opts.authUser ?? null,
      authPass: opts.authPass ?? null,
    });
    return gid;
  }, []);

  const pause = useCallback(
    (gid: string) => invoke("pause_download", { gid }) as Promise<void>,
    [],
  );
  const resume = useCallback(
    (gid: string) => invoke("resume_download", { gid }) as Promise<void>,
    [],
  );
  const remove = useCallback(
    (gid: string) => invoke("remove_download", { gid }) as Promise<void>,
    [],
  );
  const del = useCallback(
    (gid: string) => invoke("delete_download", { gid }) as Promise<void>,
    [],
  );
  const pauseAll = useCallback(
    () => invoke("pause_all") as Promise<void>,
    [],
  );
  const resumeAll = useCallback(
    () => invoke("resume_all") as Promise<void>,
    [],
  );
  const clearFinished = useCallback(
    () => invoke("clear_finished") as Promise<void>,
    [],
  );

  const updateSettings = useCallback(async (next: Settings) => {
    await invoke("update_settings", { settings: next });
    setSettings(next);
  }, []);

  const setSpeedLimit = useCallback(
    (gid: string, limit: number) =>
      invoke("set_speed_limit", { gid, limit }) as Promise<void>,
    [],
  );

  return (
    <Aria2Context.Provider
      value={{
        snapshot,
        connected,
        lastError,
        settings,
        addDownload,
        pause,
        resume,
        remove,
        del,
        pauseAll,
        resumeAll,
        clearFinished,
        updateSettings,
        setSpeedLimit,
      }}
    >
      {children}
    </Aria2Context.Provider>
  );
};

export const useAria2 = () => {
  const ctx = useContext(Aria2Context);
  if (!ctx) throw new Error("useAria2 must be used inside Aria2Provider");
  return ctx;
};

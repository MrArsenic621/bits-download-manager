import { createContext, useContext, useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Download {
  gid: string;
  uri: string;
  status: string;
  progress: number;
}

interface Aria2ContextType {
  downloads: Download[];
  addDownload: (uri: string) => Promise<void>;
}

const Aria2Context = createContext<Aria2ContextType | undefined>(undefined);

export const Aria2Provider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [downloads, setDownloads] = useState<Download[]>([]);

  const fetchDownloads = async () => {
    try {
      const list: Download[] = await invoke("get_downloads");
      setDownloads(list);
    } catch (e) {
      console.error("Failed to fetch downloads", e);
    }
  };
  const addDownload = async (uri: string) => {
    await invoke("add_download", { uri });
    // immediately refresh list after adding
    fetchDownloads();
  };

  // Poll downloads every 1-2 seconds
  useEffect(() => {
    fetchDownloads(); // initial fetch
    const interval = setInterval(fetchDownloads, 2000); // poll every 2s
    return () => clearInterval(interval); // cleanup
  }, []);

  return (
    <Aria2Context.Provider value={{ downloads, addDownload }}>
      {children}
    </Aria2Context.Provider>
  );
};

export const useAria2 = () => {
  const ctx = useContext(Aria2Context);
  if (!ctx) throw new Error("useAria2 must be inside Aria2Provider");
  return ctx;
};

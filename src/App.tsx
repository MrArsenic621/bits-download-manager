import { useState } from "react";
import { useAria2 } from "./context/Aria2Provider";
import type { Download, FilterKey } from "./types";
import Sidebar from "./components/Sidebar";
import Toolbar from "./components/Toolbar";
import DownloadList from "./components/DownloadList";
import NewDownloadModal from "./components/NewDownloadModal";
import ConfirmDialog from "./components/ConfirmDialog";
import SettingsModal from "./components/SettingsModal";

export default function App() {
  const { del, snapshot } = useAria2();
  const [filter, setFilter] = useState<FilterKey>("all");
  const [query, setQuery] = useState("");
  const [newOpen, setNewOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [pendingDelete, setPendingDelete] = useState<Download | null>(null);

  return (
    <div className="app">
      <Sidebar filter={filter} onFilter={setFilter} />

      <main className="main">
        {snapshot.startup_error && (
          <div className="error-banner">
            Download engine failed to start: {snapshot.startup_error}
          </div>
        )}
        <Toolbar
          query={query}
          onQuery={setQuery}
          onNew={() => setNewOpen(true)}
          onSettings={() => setSettingsOpen(true)}
        />
        <div className="content">
          <DownloadList
            filter={filter}
            query={query}
            onDelete={setPendingDelete}
          />
        </div>
      </main>

      {newOpen && <NewDownloadModal onClose={() => setNewOpen(false)} />}

      {settingsOpen && <SettingsModal onClose={() => setSettingsOpen(false)} />}

      {pendingDelete && (
        <ConfirmDialog
          download={pendingDelete}
          onCancel={() => setPendingDelete(null)}
          onConfirm={() => {
            void del(pendingDelete.gid);
            setPendingDelete(null);
          }}
        />
      )}
    </div>
  );
}

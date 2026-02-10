import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

export default function App() {
  const [port, setPort] = useState<number | null>(null);
  const [secret, setSecret] = useState<string | null>(null);

  useEffect(() => {
    async function fetchAria2Info() {
      try {
        const [p, s]: [number, string] = await invoke("aria2_info");
        setPort(p);
        setSecret(s);
      } catch (e) {
        console.error("Failed to get aria2 info", e);
      }
    }

    fetchAria2Info();
  }, []);

  return (
    <div className="App">
      <h1>Aria2 Info</h1>
      {port && secret ? (
        <div>
          <p>RPC Port: {port}</p>
          <p>Secret: {secret}</p>
          <p>Full RPC URL: http://127.0.0.1:{port}/jsonrpc</p>
        </div>
      ) : (
        <p>Loading aria2 info...</p>
      )}
    </div>
  );
}

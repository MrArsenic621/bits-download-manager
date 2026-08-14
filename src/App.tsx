import { useAria2 } from "./context/Aria2Provider";
import { useState } from "react";

export default function App() {
  const { downloads, addDownload } = useAria2();
  const [url, setUrl] = useState("");

  return (
    <div className="App">
      <h1>Downloads</h1>
      <input
        value={url}
        onChange={e => setUrl(e.target.value)}
        placeholder="Paste URL"
      />
      <button onClick={() => addDownload(url)}>Start Download</button>
      <ul>
        {downloads.map(d => (
          <li key={d.gid}>
            {d.uri} — {d.status} — {Math.round(d.progress)}%
          </li>
        ))}
      </ul>
    </div>
  );
}

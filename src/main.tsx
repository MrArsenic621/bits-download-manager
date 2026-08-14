import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { Aria2Provider } from "./context/Aria2Provider";
import "./App.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Aria2Provider>
    <App />
    </Aria2Provider>
  </React.StrictMode>,
);

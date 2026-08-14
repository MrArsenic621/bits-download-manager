import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { Aria2Provider } from "./context/Aria2Provider";
import { ToastProvider } from "./components/Toasts";
import "./App.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ToastProvider>
      <Aria2Provider>
        <App />
      </Aria2Provider>
    </ToastProvider>
  </React.StrictMode>,
);

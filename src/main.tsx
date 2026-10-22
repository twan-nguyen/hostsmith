import { getCurrentWindow } from "@tauri-apps/api/window";
import React from "react";
import ReactDOM from "react-dom/client";
import "./index.css";

async function start() {
  // In a plain browser during development there is no Rust side; sample data
  // stands in, so the UI can be looked at with `pnpm dev`.
  if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
    const { installMockIpc } = await import("./dev/mock-ipc");
    installMockIpc();
  }

  const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement);

  // Both windows load this page; the menu bar panel renders only its own view.
  if (getCurrentWindow().label === "panel") {
    // The panel's window is transparent around its rounded card.
    document.documentElement.style.background = "transparent";
    document.body.style.background = "transparent";
    const { MenubarPanel } = await import("./components/menubar-panel");
    root.render(
      <React.StrictMode>
        <MenubarPanel />
      </React.StrictMode>,
    );
  } else {
    const { default: App } = await import("./App");
    root.render(
      <React.StrictMode>
        <App />
      </React.StrictMode>,
    );
  }
}

void start();

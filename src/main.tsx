import { getCurrentWindow } from "@tauri-apps/api/window";
import React from "react";
import ReactDOM from "react-dom/client";
import "./index.css";

const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement);

// Both windows load this page; the menu bar panel renders only its own view.
if (getCurrentWindow().label === "panel") {
  // The panel's window is transparent around its rounded card.
  document.documentElement.style.background = "transparent";
  document.body.style.background = "transparent";
  void import("./components/menubar-panel").then(({ MenubarPanel }) =>
    root.render(
      <React.StrictMode>
        <MenubarPanel />
      </React.StrictMode>,
    ),
  );
} else {
  void import("./App").then(({ default: App }) =>
    root.render(
      <React.StrictMode>
        <App />
      </React.StrictMode>,
    ),
  );
}

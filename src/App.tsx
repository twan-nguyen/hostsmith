import { useEffect } from "react";

const darkScheme = window.matchMedia("(prefers-color-scheme: dark)");

/**
 * The window chrome only: a sidebar column under the traffic lights and the
 * working pane beside it. The panes themselves arrive with the IPC layer.
 */
function App() {
  // Follows the system appearance until the appearance setting exists.
  useEffect(() => {
    const root = document.documentElement;
    const apply = () => root.classList.toggle("dark", darkScheme.matches);
    apply();
    root.classList.add("vibrancy");
    darkScheme.addEventListener("change", apply);
    return () => darkScheme.removeEventListener("change", apply);
  }, []);

  return (
    <div className="app-canvas flex h-screen">
      <aside className="flex w-62 shrink-0 flex-col">
        {/* The traffic lights sit in this strip, which also drags the window. */}
        <div data-tauri-drag-region className="h-10 shrink-0" />
      </aside>
      <main className="flex min-w-0 flex-1 py-2 pr-2" data-tauri-drag-region>
        <section className="flex flex-1 flex-col rounded-xl border bg-card">
          <header data-tauri-drag-region className="flex h-12 shrink-0 items-center border-b px-4">
            <h1 className="font-medium">Hosts hệ thống</h1>
          </header>
        </section>
      </main>
    </div>
  );
}

export default App;

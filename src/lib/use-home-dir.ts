import { homeDir } from "@tauri-apps/api/path";
import { useEffect, useState } from "react";

// Read once per window; the home folder does not move while HostSmith runs.
let cached: string | null = null;
let pending: Promise<string | null> | null = null;

function loadHomeDir(): Promise<string | null> {
  pending ??= homeDir()
    .then((dir) => (cached = dir))
    .catch(() => null);
  return pending;
}

/** The user's home folder, or `null` until it is known (paths then show in full). */
export function useHomeDir(): string | null {
  const [home, setHome] = useState(cached);
  useEffect(() => {
    if (cached) return;
    let live = true;
    void loadHomeDir().then((dir) => live && setHome(dir));
    return () => {
      live = false;
    };
  }, []);
  return home;
}

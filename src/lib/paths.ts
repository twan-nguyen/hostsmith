/** A path under the home folder written with `~`, as the panes show it. */
export function abbreviateHome(path: string, home: string | null): string {
  if (!home) return path;
  const base = home.replace(/\/+$/, "");
  if (!base) return path;
  if (path === base) return "~";
  return path.startsWith(`${base}/`) ? `~${path.slice(base.length)}` : path;
}

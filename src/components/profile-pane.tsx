import { ArrowLeftRight, Copy, Ellipsis, FileText, Pencil, Plus, Trash2 } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";

import { Banner, PaneBanner } from "@/components/banner";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { EntryDialog } from "@/components/entry-dialog";
import { EnvironmentConflictBanner } from "@/components/environment-conflict-banner";
import { PaneEmpty } from "@/components/pane-empty";
import { Panel, PanelHeader, PendingPill } from "@/components/panel";
import { ProfileDialog } from "@/components/profile-dialog";
import { RemoveProfileDialog } from "@/components/remove-dialogs";
import { Button } from "@/components/ui/button";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Kbd } from "@/components/ui/kbd";
import { Switch } from "@/components/ui/switch";
import {
  type HostEntry,
  type Profile,
  type SwitchPreview,
  hostsApply,
  hostsDeleteEntry,
  hostsDuplicateEntry,
  hostsSetTagEnabled,
  hostsSwitchEnvironment,
  hostsSwitchPreview,
  hostsToggleEntry,
  profilesSetExclusive,
} from "@/lib/hosven";
import { activationSummary, profileBanner, profileSubtitle } from "@/lib/hosts-view";
import { overlayOpen } from "@/lib/overlay";
import { PROFILE_COLORS } from "@/lib/profile-color";
import { useSaveShortcut } from "@/lib/use-save-shortcut";
import { cn } from "@/lib/utils";
import { useStore } from "@/store";

const GRID = { gridTemplateColumns: "minmax(110px,150px) minmax(0,1.2fr) minmax(0,1fr) 40px" };

/**
 * Keys belong to whatever has focus when it takes text or runs its own
 * keymap, and to an open dialog or menu. A focused button keeps Enter and
 * Space, which press it.
 */
function keysAreTaken(event: KeyboardEvent) {
  const target = event.target;
  if (target instanceof Element) {
    if (target.closest("input, textarea, select, [contenteditable=true], .cm-editor")) return true;
    if ((event.key === "Enter" || event.key === " ") && target.closest("button, a, [role=switch]")) return true;
  }
  return overlayOpen();
}

/**
 * One profile, an entry per row. An environment is switched to from the
 * header, which writes `/etc/hosts` at once; a block has one switch for all
 * its entries. Entry edits wait for Áp dụng.
 *
 * The list is driven from the keyboard too: j/k move, Enter edits, Space
 * toggles and ⌫ deletes.
 */
export function ProfilePane({ profile }: { profile: Profile }) {
  const hosts = useStore((s) => s.hosts);
  const run = useStore((s) => s.run);
  const select = useStore((s) => s.select);

  const [editingProfile, setEditingProfile] = useState(false);
  const [deletingProfile, setDeletingProfile] = useState(false);
  const [entryForm, setEntryForm] = useState<{ entry: HostEntry | null } | null>(null);
  const [deletingEntry, setDeletingEntry] = useState<HostEntry | null>(null);
  const [cursor, setCursor] = useState(-1);
  const list = useRef<HTMLDivElement>(null);

  const entries = useMemo(() => hosts?.entries.filter((e) => e.tag === profile.name) ?? [], [hosts, profile.name]);
  const enabledCount = entries.filter((e) => e.enabled).length;
  const active = profile.isExclusive
    ? hosts?.active.kind === "active" && hosts.active.id === profile.id
    : enabledCount > 0;
  const dirty = hosts?.dirty ?? false;
  const applying = hosts?.applying ?? false;
  const color = PROFILE_COLORS[profile.color];
  const at = Math.min(cursor, entries.length - 1);

  const banner = profileBanner({ hosts, profile, entryCount: entries.length, active });

  // What activating would turn on and off, read again whenever the entries change.
  const [preview, setPreview] = useState<SwitchPreview | null>(null);
  useEffect(() => {
    if (banner !== "activation") return;
    let stale = false;
    hostsSwitchPreview(profile.id)
      .then((p) => !stale && setPreview(p))
      .catch(() => !stale && setPreview(null));
    return () => {
      stale = true;
    };
  }, [banner, hosts, profile.id]);

  const apply = () => {
    if (dirty && !applying) void run(hostsApply());
  };
  useSaveShortcut(apply);

  const edit = (entry: HostEntry) => setEntryForm({ entry });
  const toggle = (entry: HostEntry) => void run(hostsToggleEntry(entry.id));
  const copyHostname = (entry: HostEntry) => void run(navigator.clipboard.writeText(entry.hostname));

  function moveTo(index: number) {
    if (entries.length === 0) return;
    const next = Math.max(0, Math.min(entries.length - 1, index));
    setCursor(next);
    list.current?.querySelector(`[data-index="${next}"]`)?.scrollIntoView({ block: "nearest" });
  }

  // The latest values for the window listener, which is added once.
  const keys = useRef<(e: KeyboardEvent) => void>(() => {});
  keys.current = (e: KeyboardEvent) => {
    if (keysAreTaken(e) || useStore.getState().paletteOpen || e.metaKey || e.ctrlKey || e.altKey) return;
    const row = at >= 0 ? entries[at] : undefined;
    if (e.key === "j" || e.key === "ArrowDown") {
      e.preventDefault();
      moveTo(at + 1);
    } else if (e.key === "k" || e.key === "ArrowUp") {
      e.preventDefault();
      moveTo(at < 0 ? 0 : at - 1);
    } else if (!row) {
      return;
    } else if (e.key === "Enter") {
      e.preventDefault();
      edit(row);
    } else if (e.key === " ") {
      e.preventDefault();
      toggle(row);
    } else if (e.key === "Backspace" || e.key === "Delete") {
      e.preventDefault();
      setDeletingEntry(row);
    }
  };
  useEffect(() => {
    const listener = (e: KeyboardEvent) => keys.current(e);
    window.addEventListener("keydown", listener);
    return () => window.removeEventListener("keydown", listener);
  }, []);

  return (
    <Panel>
      <PanelHeader>
        <span className={cn("size-2.5 shrink-0 rounded-full", color.dot)} />
        <h1 className="truncate font-medium">{profile.name}</h1>
        <span className="shrink-0 text-xs text-muted-foreground tabular-nums">
          {profileSubtitle(profile, entries.length)}
        </span>
        <span data-testid="profile-status" className="flex shrink-0 items-center gap-1.5 text-xs text-muted-foreground">
          <span className={cn("size-1.5 rounded-full", active ? "bg-success" : "bg-muted-foreground/50")} />
          {active ? "Đang bật" : "Đang tắt"}
        </span>
        {color.environment && (
          <span className={cn("shrink-0 rounded px-1.5 py-px text-[11px] font-medium", color.chip)}>{color.label}</span>
        )}
        {dirty && <PendingPill count={hosts?.pendingCount ?? 0} testId="profile-pending" />}

        <div className="ml-auto flex shrink-0 items-center gap-2">
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="icon-sm" aria-label="Thêm" title="Thêm" data-testid="profile-menu">
                <Ellipsis />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="min-w-52">
              <DropdownMenuItem onSelect={() => setEntryForm({ entry: null })}>
                <Plus />
                Thêm entry
              </DropdownMenuItem>
              <DropdownMenuItem onSelect={() => setEditingProfile(true)}>
                <Pencil />
                Sửa
              </DropdownMenuItem>
              <DropdownMenuItem onSelect={() => void run(profilesSetExclusive(profile.id, !profile.isExclusive))}>
                <ArrowLeftRight />
                {profile.isExclusive ? "Chuyển thành block" : "Chuyển thành môi trường"}
              </DropdownMenuItem>
              <DropdownMenuItem
                disabled={entries.length === 0}
                onSelect={() => select({ kind: "hostsTag", name: profile.name })}
              >
                <FileText />
                Hiện trong Hosts hệ thống
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem variant="destructive" onSelect={() => setDeletingProfile(true)}>
                <Trash2 />
                Xoá profile
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>

          {profile.isExclusive ? (
            active ? (
              <Button
                size="sm"
                variant="outline"
                disabled={applying}
                onClick={() => void run(hostsSwitchEnvironment(null, "ask"))}
                data-testid="profile-deactivate"
              >
                Tắt
              </Button>
            ) : (
              <Button
                size="sm"
                variant={dirty ? "outline" : "default"}
                disabled={entries.length === 0 || applying}
                title={entries.length === 0 ? "Thêm entry để kích hoạt môi trường này" : undefined}
                onClick={() => void run(hostsSwitchEnvironment(profile.id, "ask"))}
                data-testid="profile-activate"
              >
                Kích hoạt
              </Button>
            )
          ) : (
            <Switch
              checked={active}
              disabled={entries.length === 0 || applying}
              onCheckedChange={(on) => void run(hostsSetTagEnabled(profile.name, on))}
              aria-label={`${profile.name}, ${active ? "bật" : "tắt"}`}
              data-testid="profile-block-toggle"
            />
          )}

          {dirty && (
            <Button size="sm" disabled={applying} onClick={apply} data-testid="profile-apply" className="animate-fade-in">
              {applying ? "Đang ghi" : "Áp dụng"}
              <Kbd className="bg-primary-foreground/15 text-primary-foreground">⌘S</Kbd>
            </Button>
          )}
        </div>
      </PanelHeader>

      <PaneBanner toast={hosts?.toast ?? null}>
        {banner === "conflict" && hosts?.active.kind === "conflict" ? (
          <EnvironmentConflictBanner ids={hosts.active.ids} />
        ) : banner === "activation" && preview ? (
          <Banner tone="info" title={activationSummary(profile.name, preview)}>
            Cần quyền quản trị một lần mỗi phiên.
          </Banner>
        ) : null}
      </PaneBanner>

      <div className="flex h-10 shrink-0 items-center gap-2 px-5 pt-1 text-xs text-muted-foreground">
        <span className="flex-1 tabular-nums">
          {entries.length === 0 ? "Entries" : `Entries · ${enabledCount} trong ${entries.length} bật`}
        </span>
        {entries.length > 1 && (
          <Button
            size="xs"
            variant="ghost"
            disabled={applying}
            onClick={() => void run(hostsSetTagEnabled(profile.name, enabledCount !== entries.length))}
          >
            {enabledCount === entries.length ? "Tắt tất cả" : "Bật tất cả"}
          </Button>
        )}
        {entries.length > 0 && (
          <Button size="xs" variant="ghost" onClick={() => setEntryForm({ entry: null })}>
            <Plus />
            Thêm entry
          </Button>
        )}
      </div>

      <div ref={list} role="grid" aria-label="Entries" className="min-h-0 flex-1 overflow-y-auto">
        {entries.length === 0 ? (
          <PaneEmpty
            title="Chưa có entry"
            detail={`Thêm hostname vào ${profile.name} để profile này có tác dụng khi kích hoạt.`}
          >
            <Button onClick={() => setEntryForm({ entry: null })}>Thêm entry</Button>
          </PaneEmpty>
        ) : (
          <>
            <div
              role="row"
              className="sticky top-0 z-10 grid h-8 items-center gap-4 border-y border-border bg-card px-5 text-xs text-muted-foreground"
              style={GRID}
            >
              <span role="columnheader">IP</span>
              <span role="columnheader">Hostname</span>
              <span role="columnheader">Ghi chú</span>
              <span role="columnheader" className="sr-only">
                Bật
              </span>
            </div>
            {entries.map((entry, index) => (
              <ContextMenu key={entry.id}>
                <ContextMenuTrigger asChild>
                  <div
                    role="row"
                    data-index={index}
                    aria-selected={at === index}
                    onClick={() => setCursor(index)}
                    onDoubleClick={() => edit(entry)}
                    onContextMenu={() => setCursor(index)}
                    data-testid={`profile-entry-${entry.hostname}`}
                    className={cn(
                      "group relative grid h-9 cursor-default items-center gap-4 border-b border-border/50 px-5 transition-colors duration-100 select-none hover:bg-accent data-[state=open]:bg-accent",
                      at === index && "bg-accent",
                    )}
                    style={GRID}
                  >
                    {at === index && (
                      <span aria-hidden className="absolute inset-y-1 left-0 w-0.5 rounded-full bg-primary" />
                    )}
                    <span
                      role="gridcell"
                      className={cn("truncate font-mono text-xs", !entry.enabled && "text-muted-foreground")}
                    >
                      {entry.ip}
                    </span>
                    <span role="gridcell" className={cn("truncate", !entry.enabled && "text-muted-foreground")}>
                      {entry.hostname}
                      {entry.aliases.length > 0 && (
                        <span className="ml-2 text-muted-foreground">{entry.aliases.join(" ")}</span>
                      )}
                    </span>
                    <span role="gridcell" className="truncate text-muted-foreground" title={entry.comment || undefined}>
                      {entry.comment}
                    </span>
                    {/* The switch acts on its own; a click there is not a row click. */}
                    <span
                      role="gridcell"
                      className="flex justify-end"
                      onClick={(e) => e.stopPropagation()}
                      onDoubleClick={(e) => e.stopPropagation()}
                    >
                      <Switch
                        size="sm"
                        checked={entry.enabled}
                        disabled={applying}
                        onCheckedChange={() => toggle(entry)}
                        aria-label={`${entry.hostname}, ${entry.enabled ? "bật" : "tắt"}`}
                        data-testid={`profile-toggle-${entry.hostname}`}
                      />
                    </span>
                  </div>
                </ContextMenuTrigger>
                <ContextMenuContent>
                  <ContextMenuItem onSelect={() => edit(entry)}>
                    <Pencil />
                    Sửa
                  </ContextMenuItem>
                  <ContextMenuItem onSelect={() => void run(hostsDuplicateEntry(entry.id))}>
                    <Copy />
                    Nhân đôi
                  </ContextMenuItem>
                  <ContextMenuItem onSelect={() => copyHostname(entry)}>Sao chép hostname</ContextMenuItem>
                  <ContextMenuSeparator />
                  <ContextMenuItem variant="destructive" onSelect={() => setDeletingEntry(entry)}>
                    <Trash2 />
                    Xoá entry
                  </ContextMenuItem>
                </ContextMenuContent>
              </ContextMenu>
            ))}
          </>
        )}
      </div>

      <EntryDialog
        open={entryForm !== null}
        entry={entryForm?.entry ?? null}
        initialTag={profile.name}
        onClose={() => setEntryForm(null)}
      />
      <ProfileDialog open={editingProfile} profile={profile} onClose={() => setEditingProfile(false)} />
      <RemoveProfileDialog profile={deletingProfile ? profile : null} onClose={() => setDeletingProfile(false)} />
      <ConfirmDialog
        open={deletingEntry !== null}
        title="Xoá entry?"
        description={`Xoá ${deletingEntry?.hostname ?? ""} khỏi ${profile.name}. Không ghi gì vào /etc/hosts cho đến khi bạn nhấn Áp dụng.`}
        confirmLabel="Xoá entry"
        onConfirm={() => deletingEntry && void run(hostsDeleteEntry(deletingEntry.id))}
        onClose={() => setDeletingEntry(null)}
      />
    </Panel>
  );
}

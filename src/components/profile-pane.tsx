import { ArrowLeftRight, Box, Copy, Ellipsis, FileText, Pencil, Plus, ToggleRight, Trash2 } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";

import { ConfirmDialog } from "@/components/confirm-dialog";
import { EntryDialog } from "@/components/entry-dialog";
import { PaneEmpty } from "@/components/pane-empty";
import { Panel, PanelHeader, PendingPill } from "@/components/panel";
import { ProfileDialog } from "@/components/profile-dialog";
import { ProfileAvatar } from "@/components/profile-avatar";
import { RemoveProfileDialog } from "@/components/remove-dialogs";
import { StatusIcon } from "@/components/status-icon";
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
  hostsApply,
  hostsDeleteEntry,
  hostsDuplicateEntry,
  hostsSetTagEnabled,
  hostsSwitchEnvironment,
  hostsToggleEntry,
  profilesSetExclusive,
} from "@/lib/hostsmith";
import { profileSubtitle } from "@/lib/hosts-view";
import { overlayOpen } from "@/lib/overlay";
import { useSaveShortcut } from "@/lib/use-save-shortcut";
import { cn } from "@/lib/utils";
import { useStore } from "@/store";

const GRID = { gridTemplateColumns: "12px minmax(0,1.3fr) minmax(110px,160px) minmax(0,1fr)" };

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
 * its entries. Entry edits wait for Apply. Messages about the file float
 * in the toaster, so the list keeps its place.
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
  const at = Math.min(cursor, entries.length - 1);

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

  const allOn = enabledCount === entries.length;

  return (
    <Panel>
      <PanelHeader>
        {profile.isExclusive ? (
          <ProfileAvatar profile={profile} />
        ) : (
          <span className="text-muted-foreground [&_svg]:size-4">
            <Box />
          </span>
        )}
        <h1 className="truncate font-medium">{profile.name}</h1>
        <span className="shrink-0 text-muted-foreground tabular-nums" title={profileSubtitle(profile, entries.length, enabledCount)}>
          {entries.length > 0 ? `${enabledCount}/${entries.length}` : ""}
        </span>
        {active && (
          <span data-testid="profile-status" className="flex h-6 shrink-0 animate-fade-in items-center gap-1.5 rounded-md px-1 text-xs text-success">
            <span className="size-1.5 rounded-full bg-success" />
            Active
          </span>
        )}

        <div className="ml-auto flex shrink-0 items-center gap-1.5">
          {dirty && <PendingPill count={hosts?.pendingCount ?? 0} testId="profile-pending" />}
          {dirty && (
            <Button size="sm" disabled={applying} onClick={apply} data-testid="profile-apply" className="animate-fade-in">
              {applying ? "Writing" : "Apply"}
              <Kbd className="bg-primary-foreground/15 text-primary-foreground">⌘S</Kbd>
            </Button>
          )}
          <Button
            size="icon-sm"
            variant="ghost"
            onClick={() => setEntryForm({ entry: null })}
            aria-label="Add entry"
            title="Add entry"
            data-testid="profile-add-entry"
            className="text-muted-foreground"
          >
            <Plus />
          </Button>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label="More actions"
                title="More actions"
                data-testid="profile-menu"
                className="text-muted-foreground"
              >
                <Ellipsis />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="min-w-56">
              <DropdownMenuItem onSelect={() => setEditingProfile(true)}>
                <Pencil />
                {profile.isExclusive ? "Edit name and color" : "Edit name"}
              </DropdownMenuItem>
              {entries.length > 1 && (
                <DropdownMenuItem
                  disabled={applying}
                  onSelect={() => void run(hostsSetTagEnabled(profile.name, !allOn))}
                >
                  <ToggleRight />
                  {allOn ? "Turn off all entries" : "Turn on all entries"}
                </DropdownMenuItem>
              )}
              <DropdownMenuItem onSelect={() => void run(profilesSetExclusive(profile.id, !profile.isExclusive))}>
                <ArrowLeftRight />
                {profile.isExclusive ? "Convert to block" : "Convert to environment"}
              </DropdownMenuItem>
              <DropdownMenuItem
                disabled={entries.length === 0}
                onSelect={() => select({ kind: "hostsTag", name: profile.name })}
              >
                <FileText />
                Show in System hosts
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem variant="destructive" onSelect={() => setDeletingProfile(true)}>
                <Trash2 />
                Delete {profile.isExclusive ? "environment" : "block"}
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
                Deactivate
              </Button>
            ) : (
              <Button
                size="sm"
                variant={dirty ? "outline" : "default"}
                disabled={entries.length === 0 || applying}
                title={
                  entries.length === 0
                    ? "Add an entry to activate this environment"
                    : "Turn this environment on, turn the others off and write /etc/hosts"
                }
                onClick={() => void run(hostsSwitchEnvironment(profile.id, "ask"))}
                data-testid="profile-activate"
              >
                Activate
              </Button>
            )
          ) : (
            <Switch
              className="ml-1"
              checked={active}
              disabled={entries.length === 0 || applying}
              onCheckedChange={(on) => void run(hostsSetTagEnabled(profile.name, on))}
              aria-label={`${profile.name}, ${active ? "on" : "off"}`}
              data-testid="profile-block-toggle"
            />
          )}
        </div>
      </PanelHeader>

      <div ref={list} role="grid" aria-label="Entries" className="min-h-0 flex-1 overflow-y-auto">
        {entries.length === 0 ? (
          <PaneEmpty
            title="No entries yet"
            detail={`Add hostnames to ${profile.name} for this ${profile.isExclusive ? "environment" : "block"} to take effect.`}
          >
            <Button onClick={() => setEntryForm({ entry: null })}>
              <Plus />
              Add entry
            </Button>
          </PaneEmpty>
        ) : (
          <>
            <div
              role="row"
              className="sticky top-0 z-10 grid h-8 items-center gap-4 border-b border-border bg-card px-5 text-xs text-muted-foreground"
              style={GRID}
            >
              {/* Holds the status column; a hidden label would leave the grid. */}
              <span role="columnheader" aria-label="On" />
              <span role="columnheader">Hostname</span>
              <span role="columnheader">Address</span>
              <span role="columnheader">Comment</span>
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
                      <span aria-hidden className="absolute inset-y-1.5 left-0 w-0.5 rounded-full bg-primary" />
                    )}
                    {/* The ring acts on its own; a click there is not a row click. */}
                    <button
                      role="gridcell"
                      onClick={(e) => {
                        e.stopPropagation();
                        toggle(entry);
                      }}
                      onDoubleClick={(e) => e.stopPropagation()}
                      disabled={applying}
                      aria-label={`${entry.hostname}, ${entry.enabled ? "on" : "off"}`}
                      aria-pressed={entry.enabled}
                      title={entry.enabled ? "Turn off entry" : "Turn on entry"}
                      data-testid={`profile-toggle-${entry.hostname}`}
                      className="-m-2 flex items-center justify-center rounded p-2 hover:[&>span]:border-success"
                    >
                      <StatusIcon on={entry.enabled} />
                    </button>
                    <span role="gridcell" className={cn("truncate", !entry.enabled && "text-muted-foreground")}>
                      {entry.hostname}
                      {entry.aliases.length > 0 && (
                        <span className="ml-2 text-muted-foreground">{entry.aliases.join(" ")}</span>
                      )}
                    </span>
                    <span role="gridcell" className="truncate font-mono text-xs text-muted-foreground">
                      {entry.ip}
                    </span>
                    <span role="gridcell" className="truncate text-muted-foreground" title={entry.comment || undefined}>
                      {entry.comment}
                    </span>
                  </div>
                </ContextMenuTrigger>
                <ContextMenuContent>
                  <ContextMenuItem onSelect={() => edit(entry)}>
                    <Pencil />
                    Edit
                  </ContextMenuItem>
                  <ContextMenuItem onSelect={() => void run(hostsDuplicateEntry(entry.id))}>
                    <Copy />
                    Duplicate
                  </ContextMenuItem>
                  <ContextMenuItem onSelect={() => copyHostname(entry)}>Copy hostname</ContextMenuItem>
                  <ContextMenuSeparator />
                  <ContextMenuItem variant="destructive" onSelect={() => setDeletingEntry(entry)}>
                    <Trash2 />
                    Delete entry
                  </ContextMenuItem>
                </ContextMenuContent>
              </ContextMenu>
            ))}
            <button
              onClick={() => setEntryForm({ entry: null })}
              className="flex h-9 w-full items-center gap-4 px-5 text-left text-muted-foreground transition-colors duration-100 hover:bg-accent hover:text-foreground [&_svg]:size-3"
            >
              <Plus />
              Add entry
            </button>
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
        title="Delete entry?"
        description={`Deletes ${deletingEntry?.hostname ?? ""} from ${profile.name}. Nothing is written to /etc/hosts until you press Apply.`}
        confirmLabel="Delete entry"
        onConfirm={() => deletingEntry && void run(hostsDeleteEntry(deletingEntry.id))}
        onClose={() => setDeletingEntry(null)}
      />
    </Panel>
  );
}

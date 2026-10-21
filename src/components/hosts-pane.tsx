import { Box, ClipboardCopy, Ellipsis, FileInput, HardDriveDownload, Network, Plus, Search, Undo2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { CodeEditor, type CodeEditorHandle } from "@/components/code-editor";
import { EntryDialog } from "@/components/entry-dialog";
import { EnvironmentConflictBanner } from "@/components/environment-conflict-banner";
import {
  ExternalChangeBanner,
  LoadErrorBanner,
  ToastBanner,
  VerificationBanner,
} from "@/components/hosts-banners";
import { ImportDialog } from "@/components/import-dialog";
import { Panel, PanelHeader } from "@/components/panel";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Kbd } from "@/components/ui/kbd";
import {
  type HostsSnapshot,
  type Toast,
  hostsApplyRaw,
  hostsBackup,
  hostsDiscard,
  hostsExport,
  hostsSetRawDraft,
  hostsSnapshot,
} from "@/lib/hosven";
import { blockMarker, draftFor, hostsBanner } from "@/lib/hosts-view";
import { useSaveShortcut } from "@/lib/use-save-shortcut";
import { useToast } from "@/lib/use-toast";
import { useStore } from "@/store";

/** Typing reaches the model this long after the last keystroke, so the pending count keeps up. */
const DRAFT_MS = 150;
/** Searching waits for a pause in typing before it moves the selection. */
const FIND_MS = 120;

/**
 * The editor's text across remounts. The model keeps unapplied typing as its
 * raw draft, but a snapshot only carries the model's own text, so the pane
 * remembers what it typed over which text and shows it again while the model
 * still holds that draft.
 */
const kept: { base: string | null; text: string } = { base: null, text: "" };

function initialText(hosts: HostsSnapshot | null) {
  if (!hosts) return "";
  return kept.base === hosts.text && hosts.dirty ? kept.text : hosts.text;
}

/**
 * `/etc/hosts` as text. The editor is the working copy: typing is handed to
 * the model as a draft, so edits made elsewhere fold it in, and Áp dụng
 * writes the editor's text as it is.
 */
export function HostsPane({ block }: { block?: string }) {
  const hosts = useStore((s) => s.hosts);
  const run = useStore((s) => s.run);
  const query = useStore((s) => s.hostsQuery);
  const setQuery = useStore((s) => s.setHostsQuery);
  const findRequest = useStore((s) => s.findRequest);
  const [toast, dismissToast] = useToast(hosts?.toast ?? null);
  const [dialog, setDialog] = useState<"entry" | "import" | null>(null);

  const editor = useRef<CodeEditorHandle>(null);
  const searchInput = useRef<HTMLInputElement>(null);
  const [text, setText] = useState(() => initialText(useStore.getState().hosts));
  /** What the editor holds now, ahead of the next render. */
  const editorText = useRef(text);
  /** The model text the editor last took in. */
  const seen = useRef<string | null>(useStore.getState().hosts?.text ?? null);
  const draftTimer = useRef<number | undefined>(undefined);
  const draftsInFlight = useRef(0);

  /** Typing the model has not seen yet; the editor is not replaced under it. */
  const typing = () => draftTimer.current !== undefined || draftsInFlight.current > 0;

  function replace(next: string) {
    editorText.current = next;
    kept.base = seen.current;
    kept.text = next;
    setText(next);
  }

  function sendDraft() {
    window.clearTimeout(draftTimer.current);
    draftTimer.current = undefined;
    const model = useStore.getState().hosts?.text;
    if (model === undefined) return Promise.resolve();
    draftsInFlight.current += 1;
    return run(hostsSetRawDraft(draftFor(editorText.current, model))).finally(() => {
      draftsInFlight.current -= 1;
    });
  }

  /** Hands typing still waiting on the debounce to the model now. */
  function flushDraft() {
    return draftTimer.current === undefined ? Promise.resolve() : sendDraft();
  }

  function onEdit(next: string) {
    // The echo of a replaced document is not typing.
    if (next === editorText.current) return;
    editorText.current = next;
    kept.base = seen.current;
    kept.text = next;
    setText(next);
    window.clearTimeout(draftTimer.current);
    draftTimer.current = window.setTimeout(() => void sendDraft(), DRAFT_MS);
  }

  // The model's text changed (apply, undo, a switch, a reload, an entry
  // edit): the editor takes it, since any draft was folded in first. A
  // discard or reload can also drop a draft without changing the text; a
  // fresh read, ordered after this pane's own drafts, confirms it before
  // the typing is replaced.
  useEffect(() => {
    if (!hosts || typing()) return;
    if (hosts.text !== seen.current) {
      seen.current = hosts.text;
      replace(hosts.text);
      return;
    }
    if (hosts.dirty || editorText.current === hosts.text) return;
    let stale = false;
    hostsSnapshot()
      .then((fresh) => {
        if (stale || typing() || fresh.dirty || editorText.current === fresh.text) return;
        seen.current = fresh.text;
        replace(fresh.text);
      })
      .catch(() => {});
    return () => {
      stale = true;
    };
  }, [hosts]);

  // Leaving the pane hands over the last keystrokes, so the model keeps them.
  useEffect(() => () => void flushDraft(), []);

  // A block opened from the sidebar shows its `## [tag:Name]` line.
  const revealed = useRef(false);
  useEffect(() => {
    if (!block || revealed.current || !text.includes(blockMarker(block))) return;
    revealed.current = true;
    editor.current?.revealLine(blockMarker(block));
  }, [block, text]);

  // The search selects the next match once typing pauses. A block pane opens
  // at its block, not at an old search.
  const openingQuery = useRef(query);
  useEffect(() => {
    if (!query || (block !== undefined && query === openingQuery.current)) return;
    const timer = window.setTimeout(() => editor.current?.find(query), FIND_MS);
    return () => window.clearTimeout(timer);
  }, [query, block]);

  // ⌘F focuses the search field.
  const lastFindRequest = useRef(findRequest);
  useEffect(() => {
    if (findRequest === lastFindRequest.current) return;
    lastFindRequest.current = findRequest;
    searchInput.current?.focus();
    searchInput.current?.select();
  }, [findRequest]);

  const applying = hosts?.applying ?? false;

  async function apply() {
    if (!hosts || applying) return;
    // A cancelled password prompt leaves the typing pending, counted by the model.
    await flushDraft();
    // A failed write reports itself through the model's toast.
    await run(hostsApplyRaw(editorText.current));
  }

  useSaveShortcut(() => void apply());

  async function discard() {
    window.clearTimeout(draftTimer.current);
    draftTimer.current = undefined;
    await run(hostsDiscard());
  }

  async function copyToClipboard() {
    const content = await run(hostsExport());
    if (content !== undefined) await run(navigator.clipboard.writeText(content));
  }

  function openDialog(which: "entry" | "import") {
    // The entry lands on top of the typing, which the model folds in first.
    void flushDraft();
    setDialog(which);
  }

  const canDiscard = !!hosts && (hosts.dirty || text !== hosts.text);

  return (
    <Panel>
      <PanelHeader>
        <span className="text-muted-foreground [&_svg]:size-4">{block ? <Box /> : <Network />}</span>
        <h1 className="truncate font-medium">{block ?? "Hosts hệ thống"}</h1>
        <span className="shrink-0 font-mono text-xs text-muted-foreground">/etc/hosts</span>
        {hosts?.dirty && (
          <span
            data-testid="hosts-pending"
            className="flex h-6 shrink-0 animate-fade-in items-center gap-1.5 rounded-md bg-warning/10 px-2 text-xs text-warning tabular-nums"
          >
            <span className="size-1.5 rounded-full bg-warning" />
            {hosts.pendingCount} chưa áp dụng
          </span>
        )}

        <label className="ml-auto flex h-7 w-56 min-w-32 shrink items-center gap-2 rounded-md px-2 text-muted-foreground transition-colors duration-100 focus-within:bg-accent hover:bg-accent">
          <Search className="size-3.5 shrink-0" />
          <input
            ref={searchInput}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                editor.current?.find(query);
              } else if (e.key === "Escape") {
                e.currentTarget.blur();
              }
            }}
            placeholder="Tìm entry"
            aria-label="Tìm entry"
            spellCheck={false}
            data-testid="hosts-search-field"
            className="min-w-0 flex-1 bg-transparent text-foreground outline-none placeholder:text-muted-foreground"
          />
          <Kbd>⌘F</Kbd>
        </label>

        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" size="icon-sm" aria-label="Thêm" title="Thêm" data-testid="hosts-menu">
              <Ellipsis />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="min-w-52">
            <DropdownMenuItem onSelect={() => openDialog("entry")}>
              <Plus />
              Thêm entry
            </DropdownMenuItem>
            <DropdownMenuItem onSelect={() => openDialog("import")}>
              <FileInput />
              Nhập entry
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem disabled={applying} onSelect={() => void run(hostsBackup())}>
              <HardDriveDownload />
              Sao lưu
            </DropdownMenuItem>
            <DropdownMenuItem onSelect={() => void copyToClipboard()}>
              <ClipboardCopy />
              Sao chép vào clipboard
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem
              variant="destructive"
              disabled={!canDiscard || applying}
              onSelect={() => void discard()}
            >
              <Undo2 />
              Huỷ thay đổi
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>

        <Button size="sm" onClick={() => void apply()} disabled={!hosts || applying} data-testid="hosts-apply">
          {applying ? "Đang ghi" : "Áp dụng"}
          <Kbd className="bg-primary-foreground/15 text-primary-foreground">⌘S</Kbd>
        </Button>
      </PanelHeader>

      <HostsBannerSlot hosts={hosts} toast={toast} onDismissToast={dismissToast} />

      <CodeEditor
        ref={editor}
        value={text}
        language="hosts"
        onChange={onEdit}
        onSave={() => void apply()}
        testId="hosts-editor"
      />

      <EntryDialog
        open={dialog === "entry"}
        entry={null}
        initialTag={block ?? null}
        onClose={() => setDialog(null)}
      />
      <ImportDialog open={dialog === "import"} onClose={() => setDialog(null)} />
    </Panel>
  );
}

function HostsBannerSlot({
  hosts,
  toast,
  onDismissToast,
}: {
  hosts: HostsSnapshot | null;
  toast: Toast | null;
  onDismissToast: () => void;
}) {
  const kind = hostsBanner(hosts, toast);
  if (!kind) return null;
  return (
    <div className="shrink-0 border-b border-border px-4 py-3">
      {kind === "toast" && toast && <ToastBanner toast={toast} onDismiss={onDismissToast} />}
      {kind === "conflict" && hosts?.active.kind === "conflict" && <EnvironmentConflictBanner ids={hosts.active.ids} />}
      {kind === "loadError" && hosts?.loadError && <LoadErrorBanner error={hosts.loadError} />}
      {kind === "externalChange" && <ExternalChangeBanner />}
      {kind === "verification" && <VerificationBanner />}
    </div>
  );
}

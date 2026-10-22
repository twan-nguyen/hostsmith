import { Box, Ellipsis, FileText, HardDriveDownload, Plus, Undo2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { CodeEditor, type CodeEditorHandle } from "@/components/code-editor";
import { EntryDialog } from "@/components/entry-dialog";
import { Panel, PanelHeader, PendingPill } from "@/components/panel";
import { PaneSearch } from "@/components/pane-search";
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
  hostsApplyRaw,
  hostsBackup,
  hostsDiscard,
  hostsSetRawDraft,
  hostsSnapshot,
} from "@/lib/hostsmith";
import { blockMarker, draftFor } from "@/lib/hosts-view";
import { useSaveShortcut } from "@/lib/use-save-shortcut";
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
 * the model as a draft, so edits made elsewhere fold it in, and Apply
 * writes the editor's text as it is.
 */
export function HostsPane({ block }: { block?: string }) {
  const hosts = useStore((s) => s.hosts);
  const run = useStore((s) => s.run);
  const query = useStore((s) => s.hostsQuery);
  const setQuery = useStore((s) => s.setHostsQuery);
  const [adding, setAdding] = useState(false);

  const editor = useRef<CodeEditorHandle>(null);
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
    const base = seen.current;
    if (base === null) return Promise.resolve();
    draftsInFlight.current += 1;
    return run(hostsSetRawDraft(draftFor(editorText.current, base), base)).finally(() => {
      draftsInFlight.current -= 1;
      // A snapshot that arrived while the draft was out was skipped.
      sync.current();
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
  const sync = useRef(() => {});
  sync.current = () => {
    const current = useStore.getState().hosts;
    if (!current || typing()) return;
    if (current.text !== seen.current) {
      seen.current = current.text;
      replace(current.text);
      return;
    }
    if (current.dirty || editorText.current === current.text) return;
    hostsSnapshot()
      .then((fresh) => {
        if (typing() || fresh.dirty || editorText.current === fresh.text) return;
        seen.current = fresh.text;
        replace(fresh.text);
      })
      .catch(() => {});
  };
  useEffect(() => sync.current(), [hosts]);

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

  const applying = hosts?.applying ?? false;

  async function apply() {
    if (!hosts || applying) return;
    // A cancelled password prompt leaves the typing pending, counted by the model.
    await flushDraft();
    // A failed write reports itself through the model's toast.
    if (seen.current !== null) await run(hostsApplyRaw(editorText.current, seen.current));
  }

  useSaveShortcut(() => void apply());

  async function discard() {
    window.clearTimeout(draftTimer.current);
    draftTimer.current = undefined;
    await run(hostsDiscard());
  }

  function addEntry() {
    // The entry lands on top of the typing, which the model folds in first.
    void flushDraft();
    setAdding(true);
  }

  const canDiscard = !!hosts && (hosts.dirty || text !== hosts.text);

  return (
    <Panel>
      <PanelHeader>
        <span className="text-muted-foreground [&_svg]:size-4">{block ? <Box /> : <FileText />}</span>
        <h1 className="truncate font-medium">{block ?? "System hosts"}</h1>
        <span className="shrink-0 font-mono text-xs text-muted-foreground">/etc/hosts</span>
        {hosts?.dirty && <PendingPill count={hosts.pendingCount} testId="hosts-pending" />}

        <PaneSearch
          value={query}
          onChange={setQuery}
          onEnter={() => editor.current?.find(query)}
          placeholder="Search entries"
          testId="hosts-search-field"
          className="ml-auto w-60"
        />

        <Button
          size="icon-sm"
          variant="ghost"
          onClick={addEntry}
          aria-label="Add entry"
          title="Add entry"
          data-testid="hosts-add-entry"
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
              data-testid="hosts-menu"
              className="text-muted-foreground"
            >
              <Ellipsis />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="min-w-52">
            <DropdownMenuItem disabled={applying} onSelect={() => void run(hostsBackup())}>
              <HardDriveDownload />
              Back up /etc/hosts
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem
              variant="destructive"
              disabled={!canDiscard || applying}
              onSelect={() => void discard()}
            >
              <Undo2 />
              Discard changes
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>

        <Button
          size="sm"
          variant={canDiscard ? "default" : "outline"}
          onClick={() => void apply()}
          disabled={!hosts || applying || !canDiscard}
          data-testid="hosts-apply"
        >
          {applying ? "Writing" : "Apply"}
          {canDiscard && <Kbd className="bg-primary-foreground/15 text-primary-foreground">⌘S</Kbd>}
        </Button>
      </PanelHeader>

      <CodeEditor
        ref={editor}
        value={text}
        onChange={onEdit}
        onSave={() => void apply()}
        readOnly={applying}
        testId="hosts-editor"
      />

      <EntryDialog open={adding} entry={null} initialTag={block ?? null} onClose={() => setAdding(false)} />
    </Panel>
  );
}

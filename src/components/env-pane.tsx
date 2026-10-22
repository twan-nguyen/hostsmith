import { Ellipsis, Plus } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useShallow } from "zustand/react/shallow";

import { Banner, type BannerAction, PaneBanner } from "@/components/banner";
import { CodeEditor, type CodeEditorHandle } from "@/components/code-editor";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { EnvKeyDialog } from "@/components/env-key-dialog";
import { type EnvProfileDialogMode, EnvProfileDialog } from "@/components/env-profile-dialog";
import { EnvTable } from "@/components/env-table";
import { PaneEmpty } from "@/components/pane-empty";
import { PaneSearch } from "@/components/pane-search";
import { Panel, PanelHeader } from "@/components/panel";
import { RemoveRepoDialog } from "@/components/remove-dialogs";
import { SegmentedControl } from "@/components/segmented-control";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { envStatus } from "@/lib/env-table";
import {
  type EnvEntry,
  type EnvRepo,
  type EnvSnapshot,
  type Exposure,
  envAddMissing,
  envApply,
  envApplyRaw,
  envDeleteEntry,
  envDiscard,
  envDiscover,
  envDismissConflict,
  envProfileApply,
  envReloadFromDisk,
  envSelect,
  envMergeRawDraft,
  envSetRawDraft,
  gitAddToGitignore,
  gitExposure,
  toolsOpenInEditor,
  toolsOpenInTerminal,
  toolsRevealInFinder,
} from "@/lib/hosven";
import { abbreviateHome, joinPath } from "@/lib/paths";
import { useHomeDir } from "@/lib/use-home-dir";
import { useSaveShortcut } from "@/lib/use-save-shortcut";
import { cn } from "@/lib/utils";
import { type EnvViewMode, useStore } from "@/store";

/** The example file is meant to be committed, so its git status is not a warning. */
const EXAMPLE_FILE = ".env.example";

/** How long the raw editor waits after the last keystroke before telling the backend. */
const DRAFT_DEBOUNCE_MS = 250;

type SavedProfile = EnvRepo["profiles"][number];

/** The selected repo's dotenv file, as a table of variables or as raw text. */
export function EnvPane() {
  const env = useStore((s) => s.env);
  const selection = useStore((s) => s.selection);
  const repoId = selection.kind === "repo" ? selection.id : selection.kind === "envFile" ? selection.repo : null;
  const repo = env?.repos.find((r) => r.id === repoId) ?? null;

  if (!env) return <Panel />;
  if (env.repos.length === 0) return <NoRepos />;
  if (!repo) {
    return (
      <Panel>
        <PaneEmpty title="Không tìm thấy repo" detail="Repo này không còn trong danh sách. Chọn một repo ở thanh bên." />
      </Panel>
    );
  }
  return <RepoPane key={repo.id} repo={repo} env={env} />;
}

function NoRepos() {
  const addRepo = useStore((s) => s.addRepo);

  return (
    <Panel>
      <PanelHeader>
        <h1 className="font-medium">Repos</h1>
      </PanelHeader>
      <PaneEmpty title="Chưa có repo" detail="Thêm thư mục của một project và Hosven sẽ tìm các file .env bên trong.">
        <Button onClick={() => void addRepo()}>Thêm repo</Button>
      </PaneEmpty>
    </Panel>
  );
}

function RepoPane({ repo, env }: { repo: EnvRepo; env: EnvSnapshot }) {
  const { tools, run, select, envQuery, setEnvQuery, envViewMode, setEnvViewMode } = useStore(
    useShallow((s) => ({
      tools: s.tools,
      run: s.run,
      select: s.select,
      envQuery: s.envQuery,
      setEnvQuery: s.setEnvQuery,
      envViewMode: s.envViewMode,
      setEnvViewMode: s.setEnvViewMode,
    })),
  );
  const home = useHomeDir();

  // The snapshot describes this repo once the backend has selected it.
  const current = env.selectedRepo === repo.id;
  const file = current ? env.selectedFile : null;
  const loaded = file !== null && !env.loadError;
  const mode = envViewMode;

  const [editing, setEditing] = useState<{ entry: EnvEntry | null } | null>(null);
  const [deleting, setDeleting] = useState<EnvEntry | null>(null);
  const [profileDialog, setProfileDialog] = useState<EnvProfileDialogMode | null>(null);
  const [confirmProfile, setConfirmProfile] = useState<SavedProfile | null>(null);
  const [removing, setRemoving] = useState(false);
  const [exposure, setExposure] = useState<Exposure>("safe");

  const editor = useRef<CodeEditorHandle>(null);

  // MARK: - Raw draft

  // What the raw editor holds when it differs from the backend's text. The
  // backend hears about it after a pause, at once when the editor loses focus,
  // and when the pane closes; each draft names its file, so it never lands in
  // another one.
  const [draft, setDraft] = useState<string | null>(null);
  const unsent = useRef<{ text: string; repo: string; file: string } | null>(null);
  const draftTimer = useRef<number | undefined>(undefined);
  const rawPending = mode === "raw" && draft !== null && draft !== env.text;

  async function flushDraft() {
    window.clearTimeout(draftTimer.current);
    const pending = unsent.current;
    unsent.current = null;
    if (!pending) return;
    // The draft names the file it was typed into, so a selection that moved
    // meanwhile does not receive it.
    await run(envSetRawDraft(pending.repo, pending.file, pending.text));
  }

  function onRawChange(text: string) {
    if (!file) return;
    setDraft(text);
    unsent.current = { text, repo: repo.id, file };
    window.clearTimeout(draftTimer.current);
    draftTimer.current = window.setTimeout(() => void flushDraft(), DRAFT_DEBOUNCE_MS);
  }

  function dropDraft() {
    window.clearTimeout(draftTimer.current);
    unsent.current = null;
    setDraft(null);
  }

  // A click anywhere outside the editor (the sidebar, a file tab) may change
  // the selection; the typing goes out first, while it still targets this file.
  const flushRef = useRef(flushDraft);
  flushRef.current = flushDraft;
  useEffect(() => {
    const onPointerDown = (e: PointerEvent) => {
      if (unsent.current && !(e.target instanceof Element && e.target.closest(".cm-editor"))) {
        void flushRef.current();
      }
    };
    window.addEventListener("pointerdown", onPointerDown, true);
    return () => {
      window.removeEventListener("pointerdown", onPointerDown, true);
      void flushRef.current();
    };
  }, []);

  // A reload, an apply, a discard or a profile replaced the text under the
  // editor: show the backend's text again.
  const previous = useRef({ text: env.text, dirty: env.dirty, file });
  useEffect(() => {
    const before = previous.current;
    previous.current = { text: env.text, dirty: env.dirty, file };
    if (before.text !== env.text || before.file !== file || (before.dirty && !env.dirty)) dropDraft();
  }, [env.text, env.dirty, file]);

  // MARK: - Mode

  async function changeMode(next: EnvViewMode) {
    if (next === mode) return;
    if (next === "table" && draft !== null) {
      // The table shows entries, so the raw text is folded into them first.
      await flushDraft();
      if (draft !== env.text) await run(envMergeRawDraft());
    }
    dropDraft();
    setEnvViewMode(next);
  }

  // MARK: - Actions

  async function apply(force = false) {
    if (!loaded || !file) return;
    if (mode === "raw") {
      // The editor's own text is written, never a draft the backend still
      // holds from before an undo.
      window.clearTimeout(draftTimer.current);
      unsent.current = null;
      await run(envApplyRaw(repo.id, file, draft ?? env.text, force));
    } else {
      await run(envApply(repo.id, file, force));
    }
  }

  /** Runs an edit after the raw editor's typing reached the backend, so it lands on top. */
  async function edit<T>(action: () => Promise<T>) {
    await flushDraft();
    return run(action());
  }

  function discard() {
    if (!file) return;
    dropDraft();
    void run(envDiscard(repo.id, file));
  }

  const repoDirty = repo.files.some((f) => f.dirty) || rawPending;

  function requestProfile(profile: SavedProfile) {
    setProfileDialog(null);
    if (repoDirty) setConfirmProfile(profile);
    else applyProfile(profile);
  }

  function applyProfile(profile: SavedProfile) {
    dropDraft();
    void run(envProfileApply(repo.id, profile.id));
  }

  // ⌘S applies. The raw editor runs its own ⌘S.
  useSaveShortcut(() => void apply());

  // In raw mode the search moves the editor to the next match.
  useEffect(() => {
    if (mode === "raw" && envQuery.trim()) editor.current?.find(envQuery.trim());
  }, [mode, envQuery]);

  // Whether git would carry the file into a commit, checked again per file.
  useEffect(() => {
    setExposure("safe");
    if (!file || file === EXAMPLE_FILE || repo.missing) return;
    let live = true;
    gitExposure(repo.path, file)
      .then((status) => live && setExposure(status))
      .catch(() => {});
    return () => {
      live = false;
    };
  }, [repo.path, repo.missing, file]);

  async function addToGitignore() {
    if (!file) return;
    const status = await run(gitAddToGitignore(repo.path, file));
    if (status) setExposure(status);
  }

  // MARK: - View

  const filePath = file ? joinPath(repo.path, file) : repo.path;
  const status = loaded ? envStatus(env.entries, env.dirty || rawPending) : null;

  function pickBanner(): React.ReactNode {
    if (!current || !file) return null;
    if (env.conflict === "changed") {
      return (
        <Banner
          tone="warning"
          title={`${file} vừa đổi bên ngoài Hosven`}
          actions={[
            { label: "Tải lại từ đĩa", onClick: () => void run(envReloadFromDisk(repo.id, file)) },
            { label: "Ghi đè bằng bản của tôi", onClick: () => void apply(true) },
          ]}
        >
          Bạn có thay đổi chưa áp dụng ở đây. Tải lại để lấy bản trên đĩa, hoặc ghi đè bằng bản của bạn.
        </Banner>
      );
    }
    if (env.conflict === "missing") {
      return (
        <Banner
          tone="warning"
          title={`${file} đã bị xoá hoặc đổi tên bên ngoài Hosven`}
          actions={[
            { label: "Tạo lại file", onClick: () => void apply(true) },
            { label: "Đóng", onClick: () => void run(envDismissConflict()) },
          ]}
        >
          Nội dung vẫn còn trong Hosven. Tạo lại file để ghi nội dung này ra đĩa.
        </Banner>
      );
    }
    if (env.loadError) {
      return (
        <Banner
          tone="danger"
          title={`Không đọc được ${file}`}
          actions={[{ label: "Thử lại", onClick: () => void run(envSelect(repo.id, file)) }]}
        >
          {env.loadError}
        </Banner>
      );
    }
    if (env.missingKeys.length > 0) {
      const count = env.missingKeys.length;
      return (
        <Banner
          tone="warning"
          title={`${count} biến trong .env.example thiếu ở đây`}
          actions={[{ label: "Thêm tất cả", onClick: () => void edit(envAddMissing) }]}
        >
          {env.missingKeys.join(", ")}
        </Banner>
      );
    }
    const gitignore: BannerAction[] = [{ label: "Thêm vào .gitignore", onClick: () => void addToGitignore() }];
    if (exposure === "tracked") {
      return (
        <Banner tone="danger" title={`${file} đang được git theo dõi`} actions={gitignore}>
          Mọi commit sẽ mang theo giá trị trong file này. Chạy{" "}
          <code className="rounded bg-muted px-1 font-mono text-xs text-foreground">git rm --cached {file}</code> rồi
          thêm vào .gitignore.
        </Banner>
      );
    }
    if (exposure === "notIgnored") {
      return (
        <Banner tone="warning" title={`${file} chưa có trong .gitignore`} actions={gitignore}>
          git add . sẽ đưa file này vào commit.
        </Banner>
      );
    }
    return null;
  }

  return (
    <Panel>
      <PanelHeader>
        <div className="flex min-w-0 flex-1 items-baseline gap-2.5">
          <h1 className="shrink-0 font-medium">{repo.name}</h1>
          <span className="min-w-0 truncate font-mono text-xs text-muted-foreground" title={filePath}>
            {abbreviateHome(filePath, home)}
          </span>
          {status && (
            <span
              className={cn(
                "flex shrink-0 items-center gap-1.5 self-center text-xs whitespace-nowrap",
                status.tone === "warning" ? "text-warning" : "text-muted-foreground",
              )}
            >
              <span className={cn("size-1.5 rounded-full", status.tone === "warning" ? "bg-warning" : "bg-success")} />
              {status.label}
            </span>
          )}
        </div>

        <PaneSearch
          value={envQuery}
          onChange={setEnvQuery}
          onEnter={() => {
            if (mode === "raw" && envQuery.trim()) editor.current?.find(envQuery.trim());
          }}
          placeholder="Tìm biến"
          disabled={!loaded}
          testId="env-search-field"
          className="w-48"
        />

        <SegmentedControl
          label="Chế độ xem"
          value={mode}
          disabled={!loaded}
          onChange={(m) => void changeMode(m)}
          options={[
            { value: "table", label: "Bảng", testId: "env-mode-table" },
            { value: "raw", label: "Raw", testId: "env-mode-raw" },
          ]}
        />

        {mode === "table" && (
          <Button
            size="icon-sm"
            variant="ghost"
            aria-label="Thêm biến"
            title="Thêm biến"
            disabled={!loaded}
            onClick={() => setEditing({ entry: null })}
          >
            <Plus />
          </Button>
        )}

        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button size="icon-sm" variant="ghost" aria-label="Thêm thao tác" title="Thêm thao tác">
              <Ellipsis />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-auto min-w-60">
            <DropdownMenuItem disabled={repo.missing} onSelect={() => setProfileDialog("save")}>
              Lưu trạng thái thành profile
            </DropdownMenuItem>
            <DropdownMenuItem onSelect={() => setProfileDialog("manage")}>Profile của repo</DropdownMenuItem>
            {(tools?.terminal || tools?.editor) && <DropdownMenuSeparator />}
            {tools?.terminal && (
              <DropdownMenuItem disabled={repo.missing} onSelect={() => void run(toolsOpenInTerminal(repo.path))}>
                Mở trong {tools.terminal.name}
              </DropdownMenuItem>
            )}
            {tools?.editor && (
              <DropdownMenuItem disabled={repo.missing} onSelect={() => void run(toolsOpenInEditor(filePath))}>
                Mở bằng {tools.editor.name}
              </DropdownMenuItem>
            )}
            <DropdownMenuSeparator />
            <DropdownMenuItem onSelect={() => void run(envDiscover(repo.id))}>Tải lại danh sách file</DropdownMenuItem>
            <DropdownMenuItem
              variant="destructive"
              disabled={!loaded || !(env.dirty || rawPending)}
              onSelect={discard}
            >
              Huỷ thay đổi
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>

        <Button size="sm" data-testid="env-apply" title="Áp dụng (⌘S)" disabled={!loaded} onClick={() => void apply()}>
          Áp dụng
        </Button>
      </PanelHeader>

      {repo.files.length > 1 && (
        <div role="tablist" aria-label="File .env" className="flex h-9 shrink-0 items-center gap-1 border-b border-border px-4">
          {repo.files.map((f) => {
            const active = f.relativePath === file;
            return (
              <button
                key={f.relativePath}
                role="tab"
                aria-selected={active}
                data-testid={`env-file-tab-${f.relativePath}`}
                onClick={() => !active && select({ kind: "envFile", repo: repo.id, path: f.relativePath })}
                className={cn(
                  "flex h-7 items-center gap-1.5 rounded-md px-2.5 font-mono text-xs transition-colors duration-100",
                  active ? "bg-accent text-foreground" : "text-muted-foreground hover:bg-accent hover:text-foreground",
                )}
              >
                {f.relativePath}
                {f.dirty && <span className="size-1.5 rounded-full bg-primary" aria-label="Chưa áp dụng" />}
              </button>
            );
          })}
        </div>
      )}

      <PaneBanner toast={env.toast}>{pickBanner()}</PaneBanner>

      <div className="flex min-h-0 flex-1 flex-col">
        {repo.discovery === "repoMissing" ? (
          <PaneEmpty
            title="Không tìm thấy thư mục"
            detail={`${abbreviateHome(repo.path, home)} đã bị chuyển hoặc xoá. Xoá repo khỏi danh sách hoặc thêm lại từ vị trí mới.`}
          >
            <Button variant="outline" onClick={() => void run(envDiscover(repo.id))}>
              Thử lại
            </Button>
            <Button variant="destructive" onClick={() => setRemoving(true)}>
              Xoá khỏi danh sách
            </Button>
          </PaneEmpty>
        ) : repo.discovery === "envMissing" ? (
          <PaneEmpty
            title="Không có file .env"
            detail={`Repo ${repo.name} không có file .env, .env.local hay .env.example ở thư mục gốc.`}
          >
            <Button variant="outline" onClick={() => void run(envDiscover(repo.id))}>
              Tải lại danh sách file
            </Button>
            <Button variant="outline" onClick={() => void run(toolsRevealInFinder(repo.path))}>
              Mở trong Finder
            </Button>
          </PaneEmpty>
        ) : !current ? null : !file ? (
          <PaneEmpty title="Chọn một file .env" detail="Chọn file ở nhóm Repos trong thanh bên." />
        ) : env.loadError ? null : mode === "raw" ? (
          // React's blur bubbles: leaving the editor sends the typing at once.
          <div className="flex min-h-0 flex-1 flex-col" onBlur={() => void flushDraft()}>
            <CodeEditor
              ref={editor}
              testId="env-editor"
              language="env"
              value={draft ?? env.text}
              onChange={onRawChange}
              onSave={() => void apply()}
            />
          </div>
        ) : (
          <EnvTable
            entries={env.entries}
            query={envQuery}
            onAdd={() => setEditing({ entry: null })}
            onEdit={(entry) => setEditing({ entry })}
            onDelete={setDeleting}
          />
        )}
      </div>

      <EnvKeyDialog
        open={editing !== null}
        entry={editing?.entry ?? null}
        file={file}
        onClose={() => setEditing(null)}
      />
      <ConfirmDialog
        open={deleting !== null}
        title="Xoá biến?"
        description={`Xoá ${deleting?.key} khỏi ${file ?? "file"}. Không ghi gì cho đến khi bạn nhấn Áp dụng.`}
        confirmLabel="Xoá biến"
        onConfirm={() => deleting && void run(envDeleteEntry(deleting.id))}
        onClose={() => setDeleting(null)}
      />
      <EnvProfileDialog
        mode={profileDialog}
        repo={repo}
        onApply={requestProfile}
        onClose={() => setProfileDialog(null)}
      />
      <ConfirmDialog
        open={confirmProfile !== null}
        title="Áp dụng profile?"
        description="Thay đổi chưa áp dụng sẽ bị bỏ. Hosven chỉ sao lưu nội dung đang có trên đĩa."
        confirmLabel="Áp dụng profile"
        onConfirm={() => confirmProfile && applyProfile(confirmProfile)}
        onClose={() => setConfirmProfile(null)}
      />
      <RemoveRepoDialog repo={removing ? repo : null} onClose={() => setRemoving(false)} />
    </Panel>
  );
}

import { open } from "@tauri-apps/plugin-dialog";
import {
  ArrowLeft,
  Box,
  Cable,
  ChevronDown,
  FileText,
  Folder,
  FolderX,
  ListChecks,
  Network,
  Plus,
  Search,
  Settings,
  SlidersHorizontal,
} from "lucide-react";
import { useRef, useState } from "react";

import appIcon from "../../src-tauri/icons/128x128.png";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { NameDialog } from "@/components/name-dialog";
import { ProfileDialog } from "@/components/profile-dialog";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { Kbd } from "@/components/ui/kbd";
import {
  type EnvRepo,
  type Profile,
  envAddRepo,
  envRemoveRepo,
  envRenameRepo,
  hostsSwitchEnvironment,
  profilesRemove,
  profilesSetExclusive,
  toolsOpenInEditor,
  toolsOpenInTerminal,
  toolsRevealInFinder,
} from "@/lib/hosven";
import { PROFILE_COLORS } from "@/lib/profile-color";
import { type Selection, sameSelection } from "@/lib/selection";
import { cn } from "@/lib/utils";
import { SIDEBAR, type SettingsPage, profileStats, useStore } from "@/store";

export function Sidebar() {
  const { selection, sidebarWidth, sidebarCollapsed, setSidebarWidth } = useStore();
  const aside = useRef<HTMLElement>(null);

  if (sidebarCollapsed) return null;

  // Dragging the right edge resizes; dragging far enough left hides the sidebar.
  const startResize = (event: React.PointerEvent) => {
    event.preventDefault();
    const left = aside.current?.getBoundingClientRect().left ?? 0;
    const move = (e: PointerEvent) => setSidebarWidth(e.clientX - left);
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      document.body.style.cursor = "";
    };
    document.body.style.cursor = "col-resize";
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  };

  return (
    <aside
      ref={aside}
      style={{ width: sidebarWidth }}
      className="relative flex shrink-0 flex-col text-sidebar-foreground"
    >
      {/* Leaves room for the macOS traffic lights and lets the window be dragged. */}
      <div data-tauri-drag-region className="h-10 shrink-0" />
      <div key={selection.kind === "settings" ? "settings" : "app"} className="flex min-h-0 flex-1 animate-view-in flex-col">
        {selection.kind === "settings" ? <SettingsNav /> : <AppNav />}
      </div>
      <div
        role="separator"
        aria-orientation="vertical"
        aria-valuemin={SIDEBAR.min}
        aria-valuemax={SIDEBAR.max}
        aria-valuenow={sidebarWidth}
        onPointerDown={startResize}
        className="absolute inset-y-0 -right-1 w-2 cursor-col-resize"
      />
    </aside>
  );
}

function AppNav() {
  const { hosts, env, ports, tools, selection, select, setPaletteOpen, openSettings, run } = useStore();
  const [profileDialog, setProfileDialog] = useState<{ profile: Profile | null } | null>(null);
  const [deleting, setDeleting] = useState<Profile | null>(null);
  const [renaming, setRenaming] = useState<EnvRepo | null>(null);
  const [removing, setRemoving] = useState<EnvRepo | null>(null);

  const stats = profileStats(hosts);
  const environments = stats.filter((s) => s.profile.isExclusive);
  const blocks = stats.filter((s) => !s.profile.isExclusive);
  const profileNames = new Set(hosts?.profiles.map((p) => p.name));
  const orphanTags = (hosts?.tags ?? []).filter((t) => !profileNames.has(t));
  const tagCount = (tag: string) => hosts?.entries.filter((e) => e.tag === tag).length ?? 0;
  const portCount = ports?.ports.filter((p) => !p.isSystem).length ?? 0;

  const switchTo = (profile: Profile, on: boolean) => {
    select({ kind: "profile", id: profile.id });
    void run(hostsSwitchEnvironment(on ? profile.id : null, "ask"));
  };

  async function addRepo() {
    const path = await open({ directory: true, title: "Chọn thư mục gốc của project" });
    if (typeof path !== "string") return;
    const before = new Set(useStore.getState().env?.repos.map((r) => r.id));
    await run(envAddRepo(path));
    const added = useStore.getState().env?.repos.find((r) => !before.has(r.id));
    if (added) select({ kind: "repo", id: added.id });
  }

  const isSelected = (s: Selection) => sameSelection(selection, s);

  return (
    <>
      <div className="flex items-center gap-1 px-3 pb-3">
        <div className="flex min-w-0 flex-1 items-center gap-2 px-1.5">
          {/* The icon art leaves macOS grid padding, so it renders a bit larger. */}
          <img src={appIcon} alt="" className="-my-1.5 -ml-1 size-8 shrink-0" />
          <span className="truncate text-[15px] font-semibold tracking-tight text-sidebar-accent-foreground">
            Hosven
          </span>
        </div>
        <IconButton label="Tìm kiếm (⌘K)" onClick={() => setPaletteOpen(true)}>
          <Search />
        </IconButton>
      </div>

      <nav className="flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto px-3 pb-3">
        <Section
          id="environments"
          title="Môi trường"
          action={{ label: "Thêm profile", onClick: () => setProfileDialog({ profile: null }) }}
        >
          {environments.length === 0 && <Hint>Chưa có môi trường</Hint>}
          {environments.map(({ profile, entryCount, active }) => (
            <ContextMenu key={profile.id}>
              <ContextMenuTrigger asChild>
                <NavItem
                  testId={`profile-row-${profile.name}`}
                  icon={<span className={cn("size-2 rounded-full", PROFILE_COLORS[profile.color].dot)} />}
                  label={profile.name}
                  active={isSelected({ kind: "profile", id: profile.id })}
                  onClick={() => select({ kind: "profile", id: profile.id })}
                  trailing={
                    active ? (
                      <Chip className={PROFILE_COLORS[profile.color].chip}>
                        {PROFILE_COLORS[profile.color].environment ? PROFILE_COLORS[profile.color].label : "Đang bật"}
                      </Chip>
                    ) : (
                      <Count value={entryCount} />
                    )
                  }
                />
              </ContextMenuTrigger>
              <ContextMenuContent>
                {active ? (
                  <ContextMenuItem onSelect={() => switchTo(profile, false)}>Tắt {profile.name}</ContextMenuItem>
                ) : (
                  <ContextMenuItem disabled={entryCount === 0} onSelect={() => switchTo(profile, true)}>
                    Chuyển sang {profile.name}
                  </ContextMenuItem>
                )}
                <ContextMenuItem onSelect={() => setProfileDialog({ profile })}>Sửa</ContextMenuItem>
                <ContextMenuItem onSelect={() => void run(profilesSetExclusive(profile.id, false))}>
                  Chuyển thành block
                </ContextMenuItem>
                <ContextMenuSeparator />
                <ContextMenuItem variant="destructive" onSelect={() => setDeleting(profile)}>
                  Xoá profile
                </ContextMenuItem>
              </ContextMenuContent>
            </ContextMenu>
          ))}
        </Section>

        <Section id="hosts" title="Hosts">
          <NavItem
            testId="sidebar-system-hosts"
            icon={<Network />}
            label="Hosts hệ thống"
            active={isSelected({ kind: "systemHosts" })}
            onClick={() => select({ kind: "systemHosts" })}
            trailing={<Count value={hosts?.entries.length ?? 0} />}
          />
          {blocks.map(({ profile, entryCount, enabledCount }) => (
            <ContextMenu key={profile.id}>
              <ContextMenuTrigger asChild>
                <NavItem
                  testId={`sidebar-block-${profile.name}`}
                  icon={<Box />}
                  label={profile.name}
                  active={isSelected({ kind: "profile", id: profile.id })}
                  onClick={() => select({ kind: "profile", id: profile.id })}
                  trailing={
                    <span className="flex items-center gap-2">
                      {enabledCount > 0 && <span className="size-1.5 rounded-full bg-success" aria-label="Đang bật" />}
                      <Count value={entryCount} />
                    </span>
                  }
                />
              </ContextMenuTrigger>
              <ContextMenuContent>
                <ContextMenuItem onSelect={() => setProfileDialog({ profile })}>Sửa</ContextMenuItem>
                <ContextMenuItem onSelect={() => void run(profilesSetExclusive(profile.id, true))}>
                  Chuyển thành môi trường
                </ContextMenuItem>
                <ContextMenuSeparator />
                <ContextMenuItem variant="destructive" onSelect={() => setDeleting(profile)}>
                  Xoá profile
                </ContextMenuItem>
              </ContextMenuContent>
            </ContextMenu>
          ))}
          {orphanTags.map((tag) => (
            <NavItem
              key={tag}
              icon={<Box />}
              label={tag}
              active={isSelected({ kind: "hostsTag", name: tag })}
              onClick={() => select({ kind: "hostsTag", name: tag })}
              trailing={<Count value={tagCount(tag)} />}
            />
          ))}
        </Section>

        <Section id="repos" title="Repos" action={{ label: "Thêm repo", onClick: () => void addRepo() }}>
          {(env?.repos.length ?? 0) === 0 && <Hint>Chưa có repo</Hint>}
          {env?.repos.map((repo) => {
            const single = repo.files.length === 1 ? repo.files[0] : null;
            const target: Selection = single
              ? { kind: "envFile", repo: repo.id, path: single.relativePath }
              : { kind: "repo", id: repo.id };
            const repoActive =
              isSelected(target) || (selection.kind === "envFile" && selection.repo === repo.id && !!single);
            return (
              <div key={repo.id} className="flex flex-col gap-px">
                <ContextMenu>
                  <ContextMenuTrigger asChild>
                    <NavItem
                      testId={`sidebar-repo-${repo.name}`}
                      icon={repo.missing ? <FolderX /> : <Folder />}
                      label={repo.name}
                      title={repo.missing ? "Không tìm thấy thư mục" : repo.path}
                      active={repoActive}
                      onClick={() => select(target)}
                      trailing={
                        repo.missing ? (
                          <span className="text-xs text-warning">Không tìm thấy</span>
                        ) : single?.dirty ? (
                          <DirtyDot />
                        ) : undefined
                      }
                    />
                  </ContextMenuTrigger>
                  <ContextMenuContent>
                    <ContextMenuItem onSelect={() => setRenaming(repo)}>Đổi tên</ContextMenuItem>
                    <ContextMenuItem disabled={repo.missing} onSelect={() => void run(toolsRevealInFinder(repo.path))}>
                      Mở trong Finder
                    </ContextMenuItem>
                    {tools?.terminal && (
                      <ContextMenuItem disabled={repo.missing} onSelect={() => void run(toolsOpenInTerminal(repo.path))}>
                        Mở trong {tools.terminal.name}
                      </ContextMenuItem>
                    )}
                    {tools?.editor && (
                      <ContextMenuItem disabled={repo.missing} onSelect={() => void run(toolsOpenInEditor(repo.path))}>
                        Mở bằng {tools.editor.name}
                      </ContextMenuItem>
                    )}
                    <ContextMenuSeparator />
                    <ContextMenuItem variant="destructive" onSelect={() => setRemoving(repo)}>
                      Xoá khỏi danh sách
                    </ContextMenuItem>
                  </ContextMenuContent>
                </ContextMenu>
                {!single &&
                  repo.files.map((file) => (
                    <div key={file.relativePath} className="pl-5">
                      <NavItem
                        testId={`sidebar-envfile-${repo.name}-${file.relativePath}`}
                        icon={<FileText />}
                        label={file.relativePath}
                        mono
                        active={isSelected({ kind: "envFile", repo: repo.id, path: file.relativePath })}
                        onClick={() => select({ kind: "envFile", repo: repo.id, path: file.relativePath })}
                        trailing={file.dirty ? <DirtyDot /> : undefined}
                      />
                    </div>
                  ))}
              </div>
            );
          })}
        </Section>

        <Section id="tools" title="Công cụ">
          <NavItem
            testId="sidebar-ports"
            icon={<Cable />}
            label="Ports"
            active={isSelected({ kind: "ports" })}
            onClick={() => select({ kind: "ports" })}
            trailing={<Count value={portCount} />}
          />
        </Section>
      </nav>

      <div className="border-t border-sidebar-border px-3 py-2">
        <button
          onClick={() => openSettings()}
          className="flex h-8 w-full items-center gap-2 rounded-md px-2 text-left transition-colors duration-100 hover:bg-sidebar-accent [&_svg]:size-4 [&_svg]:text-muted-foreground"
        >
          <Settings />
          <span className="flex-1">Cài đặt</span>
          <Kbd>⌘,</Kbd>
        </button>
      </div>

      <ProfileDialog
        open={profileDialog !== null}
        profile={profileDialog?.profile ?? null}
        onClose={() => setProfileDialog(null)}
        onCreated={(p) => select({ kind: "profile", id: p.id })}
      />
      <ConfirmDialog
        open={deleting !== null}
        title="Xoá profile?"
        description={`Xoá metadata của profile ${deleting?.name}. Các entry trong /etc/hosts được giữ nguyên.`}
        confirmLabel="Xoá profile"
        onConfirm={() => deleting && void run(profilesRemove(deleting.id))}
        onClose={() => setDeleting(null)}
      />
      <NameDialog
        open={renaming !== null}
        title="Đổi tên repo"
        label="Tên"
        initial={renaming?.name}
        submitLabel="Lưu"
        onSubmit={(name) => envRenameRepo(renaming!.id, name)}
        onClose={() => setRenaming(null)}
      />
      <ConfirmDialog
        open={removing !== null}
        title="Xoá repo khỏi danh sách?"
        description={
          removing && removing.profiles.length > 0
            ? `Các file trong ${removing.path} được giữ nguyên. ${removing.profiles.length} profile env đã lưu của repo sẽ bị xoá.`
            : `Các file trong ${removing?.path} được giữ nguyên.`
        }
        confirmLabel="Xoá khỏi danh sách"
        onConfirm={() => removing && void run(envRemoveRepo(removing.id))}
        onClose={() => setRemoving(null)}
      />
    </>
  );
}

const SETTINGS_PAGES: { page: SettingsPage; label: string; icon: React.ReactNode }[] = [
  { page: "general", label: "Chung", icon: <SlidersHorizontal /> },
  { page: "profiles", label: "Profiles", icon: <ListChecks /> },
];

function SettingsNav() {
  const { closeSettings, settingsPage, setSettingsPage } = useStore();
  return (
    <>
      <div className="px-3 pb-3">
        <button
          onClick={closeSettings}
          className="flex h-7 items-center gap-2 rounded-md px-2 text-muted-foreground transition-colors duration-100 hover:bg-sidebar-accent hover:text-sidebar-foreground [&_svg]:size-4"
        >
          <ArrowLeft />
          Quay lại
        </button>
      </div>
      <nav className="flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto px-3 pb-3">
        <Section id="settings" title="Cài đặt">
          {SETTINGS_PAGES.map(({ page, label, icon }) => (
            <NavItem
              key={page}
              icon={icon}
              label={label}
              active={settingsPage === page}
              onClick={() => setSettingsPage(page)}
            />
          ))}
        </Section>
      </nav>
    </>
  );
}

function Section({
  id,
  title,
  action,
  children,
}: {
  id: string;
  title: string;
  action?: { label: string; onClick: () => void };
  children: React.ReactNode;
}) {
  const { collapsedGroups, toggleGroup } = useStore();
  const open = !collapsedGroups.includes(id);
  return (
    <div className="group/section flex flex-col gap-px">
      <div className="flex items-center justify-between">
        <button
          onClick={() => toggleGroup(id)}
          aria-expanded={open}
          className="flex h-6 w-fit items-center gap-1 rounded-md px-2 text-xs font-medium text-muted-foreground transition-colors duration-150 hover:text-sidebar-foreground"
        >
          {title}
          <ChevronDown
            className={cn("size-3 transition-transform duration-200 ease-out-quart", !open && "-rotate-90")}
          />
        </button>
        {action && (
          <button
            onClick={action.onClick}
            aria-label={action.label}
            title={action.label}
            className="mr-1 flex size-5 items-center justify-center rounded-md text-muted-foreground opacity-0 transition-opacity duration-100 group-hover/section:opacity-100 hover:bg-sidebar-accent hover:text-sidebar-foreground focus-visible:opacity-100 [&_svg]:size-3.5"
          >
            <Plus />
          </button>
        )}
      </div>
      <Collapsible open={open}>{children}</Collapsible>
    </div>
  );
}

/** Animating grid rows from 1fr to 0fr collapses to the content's real height. */
function Collapsible({ open, children }: { open: boolean; children: React.ReactNode }) {
  return (
    <div
      inert={!open}
      className={cn(
        "grid transition-[grid-template-rows,opacity] duration-200 ease-out-quart",
        open ? "grid-rows-[1fr] opacity-100" : "grid-rows-[0fr] opacity-0",
      )}
    >
      <div className="flex min-h-0 flex-col gap-px overflow-hidden">{children}</div>
    </div>
  );
}

function NavItem({
  icon,
  label,
  title,
  trailing,
  active,
  mono,
  testId,
  onClick,
  ...props
}: {
  icon: React.ReactNode;
  label: string;
  title?: string;
  trailing?: React.ReactNode;
  active?: boolean;
  mono?: boolean;
  testId?: string;
  onClick?: () => void;
} & Omit<React.ComponentProps<"button">, "onClick">) {
  return (
    <button
      {...props}
      onClick={onClick}
      title={title ?? label}
      data-testid={testId}
      aria-current={active ? "page" : undefined}
      className={cn(
        "flex h-7 w-full items-center gap-2.5 rounded-md px-2 text-left transition-colors duration-100 hover:bg-sidebar-accent data-[state=open]:bg-sidebar-accent [&_svg]:size-4 [&_svg]:shrink-0 [&_svg]:text-muted-foreground",
        active && "bg-sidebar-accent text-sidebar-accent-foreground",
      )}
    >
      <span className="flex size-4 shrink-0 items-center justify-center">{icon}</span>
      <span className={cn("flex-1 truncate", mono && "font-mono text-xs")}>{label}</span>
      {trailing}
    </button>
  );
}

function Count({ value }: { value: number }) {
  return value > 0 ? <span className="text-xs text-muted-foreground tabular-nums">{value}</span> : null;
}

function Chip({ className, children }: { className?: string; children: React.ReactNode }) {
  return <span className={cn("rounded px-1.5 py-px text-[11px] font-medium", className)}>{children}</span>;
}

function DirtyDot() {
  return <span className="size-1.5 rounded-full bg-primary" aria-label="Chưa áp dụng" />;
}

function Hint({ children }: { children: React.ReactNode }) {
  return <p className="px-2 py-1 text-xs text-muted-foreground">{children}</p>;
}

function IconButton({
  label,
  onClick,
  children,
}: {
  label: string;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      aria-label={label}
      title={label}
      className="flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground transition-colors duration-100 hover:bg-sidebar-accent hover:text-sidebar-foreground [&_svg]:size-4"
    >
      {children}
    </button>
  );
}


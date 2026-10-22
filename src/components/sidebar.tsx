import { ArrowLeft, Box, Cable, Check, ChevronDown, ChevronsUpDown, FileText, Plus, Power, Search, Settings } from "lucide-react";
import { useRef, useState } from "react";
import { useShallow } from "zustand/react/shallow";

import appIcon from "../../src-tauri/icons/128x128.png";
import { ProfileAvatar } from "@/components/profile-avatar";
import { ProfileDialog } from "@/components/profile-dialog";
import { RemoveProfileDialog } from "@/components/remove-dialogs";
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
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuShortcut,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Kbd } from "@/components/ui/kbd";
import { profileStats } from "@/lib/hosts-view";
import { type Profile, hostsSwitchEnvironment, profilesSetExclusive } from "@/lib/hostsmith";
import { type Selection, sameSelection } from "@/lib/selection";
import { cn } from "@/lib/utils";
import { SIDEBAR, useStore } from "@/store";

export function Sidebar() {
  const inSettings = useStore((s) => s.selection.kind === "settings");
  const sidebarWidth = useStore((s) => s.sidebarWidth);
  const sidebarCollapsed = useStore((s) => s.sidebarCollapsed);
  const setSidebarWidth = useStore((s) => s.setSidebarWidth);
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
    <aside ref={aside} style={{ width: sidebarWidth }} className="relative flex shrink-0 flex-col text-sidebar-foreground">
      {/* Leaves room for the macOS traffic lights and lets the window be dragged. */}
      <div data-tauri-drag-region className="h-10 shrink-0" />
      <div key={inSettings ? "settings" : "app"} className="flex min-h-0 flex-1 animate-view-in flex-col">
        {inSettings ? <SettingsNav /> : <AppNav />}
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

/**
 * The two views of the file, then every environment and block by name. The
 * switcher at the foot answers the question the app exists for: which
 * environment `/etc/hosts` points at.
 */
function AppNav() {
  const { hosts, portCount, selection, select, setPaletteOpen, openSettings, run } = useStore(
    useShallow((s) => ({
      hosts: s.hosts,
      portCount: s.ports?.ports.filter((p) => p.owner.kind === "yours").length ?? 0,
      selection: s.selection,
      select: s.select,
      setPaletteOpen: s.setPaletteOpen,
      openSettings: s.openSettings,
      run: s.run,
    })),
  );
  const [profileDialog, setProfileDialog] = useState<{ profile: Profile | null; exclusive: boolean } | null>(null);
  const [deleting, setDeleting] = useState<Profile | null>(null);

  const stats = profileStats(hosts);
  const environments = stats.filter((s) => s.profile.isExclusive);
  const blocks = stats.filter((s) => !s.profile.isExclusive);
  const profileNames = new Set(hosts?.profiles.map((p) => p.name));
  const orphanTags = (hosts?.tags ?? []).filter((t) => !profileNames.has(t));

  const isSelected = (s: Selection) => sameSelection(selection, s);
  const switchTo = (profile: Profile | null) => void run(hostsSwitchEnvironment(profile?.id ?? null, "ask"));

  const profileMenu = (profile: Profile, active: boolean, entryCount: number) => (
    <ContextMenuContent>
      {profile.isExclusive &&
        (active ? (
          <ContextMenuItem onSelect={() => switchTo(null)}>Tắt {profile.name}</ContextMenuItem>
        ) : (
          <ContextMenuItem disabled={entryCount === 0} onSelect={() => switchTo(profile)}>
            Chuyển sang {profile.name}
          </ContextMenuItem>
        ))}
      <ContextMenuItem onSelect={() => setProfileDialog({ profile, exclusive: profile.isExclusive })}>Sửa</ContextMenuItem>
      <ContextMenuItem onSelect={() => void run(profilesSetExclusive(profile.id, !profile.isExclusive))}>
        {profile.isExclusive ? "Chuyển thành block" : "Chuyển thành môi trường"}
      </ContextMenuItem>
      <ContextMenuSeparator />
      <ContextMenuItem variant="destructive" onSelect={() => setDeleting(profile)}>
        Xoá
      </ContextMenuItem>
    </ContextMenuContent>
  );

  return (
    <>
      <div className="flex items-center gap-1 px-3 pb-3">
        <div className="flex min-w-0 flex-1 items-center gap-2 px-1.5">
          {/* The icon art leaves macOS grid padding, so it renders a bit larger. */}
          <img src={appIcon} alt="" className="-my-1.5 -ml-1 size-8 shrink-0" />
          {/* Wordmark: the app's own word stays bright, the shared brand recedes. */}
          <span className="truncate text-[15px] font-semibold tracking-tight text-sidebar-accent-foreground">
            Host<span className="font-medium text-sidebar-accent-foreground/35">Smith</span>
          </span>
        </div>
        <IconButton label="Tìm kiếm (⌘K)" onClick={() => setPaletteOpen(true)}>
          <Search />
        </IconButton>
      </div>

      <nav className="flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto px-3 pb-3">
        <div className="flex flex-col gap-px">
          <NavItem
            testId="sidebar-system-hosts"
            icon={<FileText />}
            label="Hosts hệ thống"
            active={isSelected({ kind: "systemHosts" })}
            onClick={() => select({ kind: "systemHosts" })}
            trailing={hosts?.dirty ? <Dot className="bg-warning" label="Có thay đổi chưa áp dụng" /> : null}
          />
          <NavItem
            testId="sidebar-ports"
            icon={<Cable />}
            label="Cổng"
            active={isSelected({ kind: "ports" })}
            onClick={() => select({ kind: "ports" })}
            trailing={portCount > 0 ? <Count value={portCount} /> : null}
          />
        </div>

        <Section
          id="environments"
          title="Môi trường"
          onAdd={() => setProfileDialog({ profile: null, exclusive: true })}
          addLabel="Thêm môi trường"
        >
          {environments.length === 0 && <Hint>Chưa có môi trường nào</Hint>}
          {environments.map(({ profile, entryCount, active }) => (
            <ContextMenu key={profile.id}>
              <ContextMenuTrigger asChild>
                <NavItem
                  testId={`profile-row-${profile.name}`}
                  icon={<ProfileAvatar profile={profile} />}
                  label={profile.name}
                  active={isSelected({ kind: "profile", id: profile.id })}
                  onClick={() => select({ kind: "profile", id: profile.id })}
                  trailing={
                    active ? (
                      <Dot className="bg-success" label="Đang bật" />
                    ) : profile.shortcutNumber ? (
                      <Kbd className="opacity-0 transition-opacity group-hover/item:opacity-100">
                        ⌘{profile.shortcutNumber}
                      </Kbd>
                    ) : null
                  }
                />
              </ContextMenuTrigger>
              {profileMenu(profile, active, entryCount)}
            </ContextMenu>
          ))}
        </Section>

        <Section
          id="blocks"
          title="Block"
          onAdd={() => setProfileDialog({ profile: null, exclusive: false })}
          addLabel="Thêm block"
        >
          {blocks.length === 0 && orphanTags.length === 0 && <Hint>Chưa có block nào</Hint>}
          {blocks.map(({ profile, entryCount, enabledCount }) => (
            <ContextMenu key={profile.id}>
              <ContextMenuTrigger asChild>
                <NavItem
                  testId={`sidebar-block-${profile.name}`}
                  icon={<Box />}
                  label={profile.name}
                  active={isSelected({ kind: "profile", id: profile.id })}
                  onClick={() => select({ kind: "profile", id: profile.id })}
                  trailing={enabledCount > 0 ? <Dot className="bg-success" label="Đang bật" /> : null}
                />
              </ContextMenuTrigger>
              {profileMenu(profile, enabledCount > 0, entryCount)}
            </ContextMenu>
          ))}
          {orphanTags.map((tag) => (
            <NavItem
              key={tag}
              icon={<Box />}
              label={tag}
              active={isSelected({ kind: "hostsTag", name: tag })}
              onClick={() => select({ kind: "hostsTag", name: tag })}
            />
          ))}
        </Section>
      </nav>

      <div className="border-t border-sidebar-border px-3 py-2">
        <EnvironmentSwitcher environments={environments} onSwitch={switchTo} onSettings={() => openSettings()} />
      </div>

      <ProfileDialog
        open={profileDialog !== null}
        profile={profileDialog?.profile ?? null}
        exclusive={profileDialog?.exclusive ?? true}
        onClose={() => setProfileDialog(null)}
        onCreated={(p) => select({ kind: "profile", id: p.id })}
      />
      <RemoveProfileDialog profile={deleting} onClose={() => setDeleting(null)} />
    </>
  );
}

type EnvironmentStat = ReturnType<typeof profileStats>[number];

/**
 * Sits at the foot of the sidebar, like the workspace switcher in Slack:
 * which environment `/etc/hosts` points at, and the way to another one.
 */
function EnvironmentSwitcher({
  environments,
  onSwitch,
  onSettings,
}: {
  environments: EnvironmentStat[];
  onSwitch: (profile: Profile | null) => void;
  onSettings: () => void;
}) {
  const applying = useStore((s) => s.hosts?.applying ?? false);
  const on = environments.filter((e) => e.active);
  const current = on.length === 1 ? on[0].profile : null;
  const label = applying ? "Đang ghi /etc/hosts" : current ? current.name : on.length > 1 ? `${on.length} môi trường cùng bật` : "Không bật môi trường";

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button
          data-testid="environment-switcher"
          className="flex h-8 w-full min-w-0 items-center gap-2 rounded-md px-2 text-left transition-colors duration-100 hover:bg-sidebar-accent data-[state=open]:bg-sidebar-accent"
        >
          {current ? (
            <ProfileAvatar profile={current} />
          ) : (
            <span
              className={cn(
                "size-4 shrink-0 rounded border border-dashed",
                on.length > 1 ? "border-warning" : "border-muted-foreground/50",
              )}
            />
          )}
          <span className={cn("flex-1 truncate", !current && "text-muted-foreground")}>{label}</span>
          <ChevronsUpDown className="size-3.5 shrink-0 text-muted-foreground" />
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent side="top" align="start" className="w-(--radix-dropdown-menu-trigger-width) min-w-56">
        <DropdownMenuLabel>Môi trường</DropdownMenuLabel>
        {environments.length === 0 && <DropdownMenuItem disabled>Chưa có môi trường nào</DropdownMenuItem>}
        {environments.map(({ profile, entryCount, active }) => (
          <DropdownMenuItem
            key={profile.id}
            disabled={entryCount === 0 || applying}
            title={entryCount === 0 ? "Chưa có entry" : undefined}
            onSelect={() => !active && onSwitch(profile)}
          >
            <ProfileAvatar profile={profile} />
            <span className="flex-1 truncate">{profile.name}</span>
            {active && <Check />}
            {profile.shortcutNumber && <DropdownMenuShortcut>⌘{profile.shortcutNumber}</DropdownMenuShortcut>}
          </DropdownMenuItem>
        ))}
        <DropdownMenuItem disabled={on.length === 0 || applying} onSelect={() => onSwitch(null)}>
          <Power />
          <span className="flex-1">Tắt môi trường</span>
          <DropdownMenuShortcut>⌘0</DropdownMenuShortcut>
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem onSelect={onSettings}>
          <Settings />
          Cài đặt
          <DropdownMenuShortcut>⌘,</DropdownMenuShortcut>
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** Settings fill the pane; the sidebar only leads back. */
function SettingsNav() {
  const closeSettings = useStore((s) => s.closeSettings);
  return (
    <div className="px-3 pb-3">
      <button
        onClick={closeSettings}
        className="flex h-7 items-center gap-2 rounded-md px-2 text-muted-foreground transition-colors duration-100 hover:bg-sidebar-accent hover:text-sidebar-foreground [&_svg]:size-4"
      >
        <ArrowLeft />
        Quay lại
      </button>
    </div>
  );
}

function Section({
  id,
  title,
  onAdd,
  addLabel,
  children,
}: {
  id: string;
  title: string;
  onAdd?: () => void;
  addLabel?: string;
  children: React.ReactNode;
}) {
  const open = useStore((s) => !s.collapsedGroups.includes(id));
  const toggleGroup = useStore((s) => s.toggleGroup);
  return (
    <div className="group/section flex flex-col gap-px">
      <div className="flex h-6 items-center justify-between">
        <button
          onClick={() => toggleGroup(id)}
          aria-expanded={open}
          className="flex h-6 items-center gap-1 rounded-md px-2 text-xs font-medium text-muted-foreground transition-colors duration-150 hover:text-sidebar-foreground"
        >
          {title}
          <ChevronDown className={cn("size-3 transition-transform duration-200 ease-out-quart", !open && "-rotate-90")} />
        </button>
        {onAdd && (
          <button
            onClick={onAdd}
            aria-label={addLabel}
            title={addLabel}
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
  trailing,
  active,
  testId,
  onClick,
  ...props
}: {
  icon: React.ReactNode;
  label: string;
  trailing?: React.ReactNode;
  active?: boolean;
  testId?: string;
  onClick?: () => void;
} & Omit<React.ComponentProps<"button">, "onClick">) {
  return (
    <button
      {...props}
      onClick={onClick}
      title={label}
      data-testid={testId}
      aria-current={active ? "page" : undefined}
      className={cn(
        "group/item flex h-7 w-full items-center gap-2.5 rounded-md px-2 text-left transition-colors duration-100 hover:bg-sidebar-accent data-[state=open]:bg-sidebar-accent [&_svg]:size-4 [&_svg]:shrink-0 [&_svg]:text-muted-foreground",
        active && "bg-sidebar-accent text-sidebar-accent-foreground",
      )}
    >
      <span className="flex size-4 shrink-0 items-center justify-center">{icon}</span>
      <span className="flex-1 truncate">{label}</span>
      {trailing}
    </button>
  );
}

function Count({ value }: { value: number }) {
  return <span className="text-xs text-muted-foreground tabular-nums">{value}</span>;
}

function Dot({ className, label }: { className: string; label: string }) {
  return <span aria-label={label} title={label} className={cn("mr-1 size-1.5 shrink-0 rounded-full", className)} />;
}

function Hint({ children }: { children: React.ReactNode }) {
  return <p className="px-2 py-1 text-xs text-muted-foreground">{children}</p>;
}

function IconButton({ label, onClick, children }: { label: string; onClick: () => void; children: React.ReactNode }) {
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

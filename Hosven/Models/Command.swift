import Foundation
import SwiftUI

/// Category bucket for the ⌘K command palette. Order in `displayOrder` controls
/// which sections render first.
enum CommandCategory: String, CaseIterable {
    case profileActions
    case suggestions

    var title: String {
        switch self {
        case .profileActions: return "Profile"
        case .suggestions:    return "Gợi ý"
        }
    }

    static let displayOrder: [CommandCategory] = [.profileActions, .suggestions]
}

/// Anything executable from the palette. `id` must be stable so SwiftUI
/// keeps focus across query updates when a command stays in the result set.
protocol PaletteCommand {
    var id: String { get }
    var title: String { get }
    var subtitle: String? { get }
    var icon: String { get }
    var iconTint: Color { get }
    var category: CommandCategory { get }
    var shortcutHint: String? { get }
    @MainActor func execute(in context: PaletteContext)
}

/// Side-effects the palette can trigger. Held by the host view so commands
/// don't need direct references to managers.
@MainActor
struct PaletteContext {
    let hostsManager: HostsFileManager
    let envManager: EnvFileManager
    let select: (AppSidebarSelection) -> Void
    let dismiss: () -> Void
}

// MARK: - Concrete commands

struct SwitchProfileCommand: PaletteCommand {
    let profile: Profile
    let isActive: Bool

    var id: String { "switch-profile-\(profile.id.uuidString)" }
    var title: String { "Chuyển sang \(profile.name)" }
    var subtitle: String? { isActive ? "Đang hoạt động" : nil }
    var icon: String { "square.stack.3d.up" }
    var iconTint: Color { Color.ds(profile.color) }
    var category: CommandCategory { .profileActions }
    var shortcutHint: String? {
        guard let n = profile.shortcutNumber, (1...9).contains(n) else { return nil }
        return "⌘\(n)"
    }

    func execute(in context: PaletteContext) {
        context.select(.profile(profile.id))
        context.hostsManager.switchEnvironment(to: profile.id, policy: .askIfPending)
        context.dismiss()
    }
}

struct ClearProfileCommand: PaletteCommand {
    var id: String { "clear-profile" }
    var title: String { "Tắt môi trường đang bật" }
    var subtitle: String? { "Ghi vào /etc/hosts" }
    var icon: String { "circle.dashed" }
    var iconTint: Color { DS.Colors.inkSecondary }
    var category: CommandCategory { .profileActions }
    var shortcutHint: String? { "⌘0" }

    func execute(in context: PaletteContext) {
        context.hostsManager.switchEnvironment(to: nil, policy: .askIfPending)
        context.dismiss()
    }
}

/// Jumps to a sidebar item: System hosts or a repo.
struct OpenSelectionCommand: PaletteCommand {
    let selection: AppSidebarSelection
    let title: String
    let subtitle: String?
    let icon: String

    var id: String { "open-\(selection.rawValue)" }
    var iconTint: Color { DS.Colors.accent }
    var category: CommandCategory { .suggestions }
    var shortcutHint: String? { nil }

    func execute(in context: PaletteContext) {
        context.select(selection)
        context.dismiss()
    }
}

/// Pushes the current palette query into the Hosts tab search field.
/// Always offered when query is non-empty so user can hop straight to filtered results.
struct SearchInHostsCommand: PaletteCommand {
    let query: String

    var id: String { "search-hosts" }
    var title: String { "Tìm \"\(query)\" trong Hosts" }
    var subtitle: String? { "Lọc theo hostname / IP" }
    var icon: String { "magnifyingglass" }
    var iconTint: Color { DS.Colors.accent }
    var category: CommandCategory { .suggestions }
    var shortcutHint: String? { nil }

    func execute(in context: PaletteContext) {
        context.select(.systemHosts)
        context.hostsManager.pendingSearchQuery = query
        context.dismiss()
    }
}

/// Pushes the current palette query into the Env file pane search field.
struct SearchInEnvCommand: PaletteCommand {
    let query: String

    var id: String { "search-env" }
    var title: String { "Tìm \"\(query)\" trong Env" }
    var subtitle: String? { "Lọc theo key / value" }
    var icon: String { "magnifyingglass" }
    var iconTint: Color { DS.Colors.accent }
    var category: CommandCategory { .suggestions }
    var shortcutHint: String? { nil }

    func execute(in context: PaletteContext) {
        if let repoId = context.envManager.selectedRepoId ?? context.envManager.repos.first?.id {
            context.select(.repo(repoId))
        }
        context.envManager.pendingSearchQuery = query
        context.dismiss()
    }
}


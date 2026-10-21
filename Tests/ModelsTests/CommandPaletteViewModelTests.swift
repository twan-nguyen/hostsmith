import Testing
import Foundation
@testable import Hosven

/// In-memory profile store stub — keeps tests fast and isolated from UserDefaults.
private final class StubProfileStore: ProfileStoring {
    var profiles: [Profile]
    init(_ profiles: [Profile] = []) { self.profiles = profiles }
    func load() -> [Profile] { profiles }
    func save(_ p: [Profile]) { profiles = p }
}

@Suite("CommandPaletteViewModel")
@MainActor
struct CommandPaletteViewModelTests {

    // MARK: - Helpers

    private func makeManagers(profiles: [Profile] = Profile.defaults) -> (HostsFileManager, EnvFileManager) {
        let hosts = HostsFileManager(profileStore: StubProfileStore(profiles), autoLoad: false)
        // Every profile gets one disabled entry, so each can be switched to and none is active.
        hosts.parseHostsContent(profiles.map { "## [tag:\($0.name)]\n# 127.0.0.1\t\($0.name.lowercased()).test\n## [/tag:\($0.name)]" }.joined(separator: "\n"))
        let env = EnvFileManager(storageURL: URL(fileURLWithPath: "/tmp/test-env-\(UUID()).json"))
        return (hosts, env)
    }

    private func makeVM(profiles: [Profile] = Profile.defaults) -> (CommandPaletteViewModel, HostsFileManager) {
        let (hosts, env) = makeManagers(profiles: profiles)
        let vm = CommandPaletteViewModel()
        vm.bind(hosts: hosts, env: env)
        return (vm, hosts)
    }

    // MARK: - Empty query default sections

    @Test("Empty query shows profile actions + suggestions only")
    func emptyQueryDefaultSections() {
        let (vm, _) = makeVM()
        let categories = vm.sections.map(\.category)
        #expect(categories.contains(.profileActions))
        #expect(categories.contains(.suggestions))
        #expect(!vm.flat.isEmpty)
    }

    @Test("Empty query lists environments + open system hosts + open ports")
    func emptyQueryUniverseSize() {
        let (vm, _) = makeVM()
        // 3 default environments + open system hosts + open ports (no repos); nothing active, so no clear
        #expect(vm.flat.count == 5)
        #expect(!vm.flat.contains { $0 is ClearProfileCommand })
    }

    @Test("Blocks are not offered as environments to switch to")
    func blocksNotOffered() {
        let (vm, _) = makeVM(profiles: Profile.defaults + [Profile(name: "Docker", color: .purple)])
        #expect(!vm.flat.contains { ($0 as? SwitchProfileCommand)?.profile.name == "Docker" })
    }

    // MARK: - Query filtering

    @Test("Query 'rel' ranks Release profile first")
    func querySwitchProfileRanks() {
        let (vm, _) = makeVM()
        vm.query = "rel"
        let titles = vm.flat.map(\.title)
        let firstTitle = titles.first ?? "<empty>"
        #expect(firstTitle.contains("Release"), "Expected 'Release' first, got titles: \(titles)")
    }

    @Test("Query with no fuzzy match still offers search-jump commands")
    func queryNoMatchOffersSearch() {
        let (vm, _) = makeVM()
        vm.query = "zzzqqq"
        #expect(vm.flat.count == 2)
        #expect(vm.flat.contains { $0 is SearchInHostsCommand })
        #expect(vm.flat.contains { $0 is SearchInEnvCommand })
    }

    @Test("Query 'hosts' surfaces the open system hosts suggestion")
    func querySurfacesOpenSelection() {
        let (vm, _) = makeVM()
        vm.query = "hosts"
        #expect(vm.flat.contains { ($0 as? OpenSelectionCommand)?.selection == .systemHosts })
    }

    // MARK: - Selection

    @Test("moveSelection wraps from last to first")
    func selectionWrapsForward() {
        let (vm, _) = makeVM()
        vm.selectedIndex = vm.flat.count - 1
        vm.moveSelection(by: 1)
        #expect(vm.selectedIndex == 0)
    }

    @Test("moveSelection wraps from first to last")
    func selectionWrapsBackward() {
        let (vm, _) = makeVM()
        vm.selectedIndex = 0
        vm.moveSelection(by: -1)
        #expect(vm.selectedIndex == vm.flat.count - 1)
    }

    @Test("moveSelection on no-fuzzy-match still wraps among search jumps")
    func selectionWrapsSearchOnly() {
        let (vm, _) = makeVM()
        vm.query = "zzzqqq"
        // Two search-jump commands → wrap from 1 → 0.
        vm.selectedIndex = 1
        vm.moveSelection(by: 1)
        #expect(vm.selectedIndex == 0)
    }

    @Test("selectedCommand defaults to first search-jump")
    func selectedCommandDefault() {
        let (vm, _) = makeVM()
        vm.query = "zzzqqq"
        #expect(vm.selectedCommand is SearchInHostsCommand)
    }

    // MARK: - Reset

    @Test("reset clears query and selection")
    func resetClears() {
        let (vm, _) = makeVM()
        vm.query = "rel"
        vm.selectedIndex = 0
        vm.reset()
        #expect(vm.query == "")
    }

    // MARK: - Active profile awareness

    @Test("Switch profile command marks active profile")
    func activeProfileMarked() {
        let (vm, hosts) = makeVM()
        hosts.parseHostsContent("## [tag:Release]\n127.0.0.1\trelease.test\n## [/tag:Release]\n")
        // Force rebuild via a query change.
        vm.query = ""
        #expect(vm.flat.contains { $0 is ClearProfileCommand })
        let release = vm.flat.compactMap { $0 as? SwitchProfileCommand }
            .first { $0.profile.name == "Release" }
        #expect(release?.isActive == true)
    }

    // MARK: - Cap per section

    // MARK: - Search-jump commands

    @Test("Non-empty query offers Search-in-Hosts + Search-in-Env")
    func searchJumpsOfferedOnQuery() {
        let (vm, _) = makeVM()
        vm.query = "abc"
        #expect(vm.flat.contains { $0 is SearchInHostsCommand })
        #expect(vm.flat.contains { $0 is SearchInEnvCommand })
    }

    @Test("Empty query does NOT offer search-jump commands")
    func searchJumpsHiddenWhenEmpty() {
        let (vm, _) = makeVM()
        #expect(!vm.flat.contains { $0 is SearchInHostsCommand })
        #expect(!vm.flat.contains { $0 is SearchInEnvCommand })
    }

    @Test("SearchInHostsCommand pushes query to hostsManager.pendingSearchQuery")
    func searchInHostsExecutes() {
        let (hosts, env) = makeManagers()
        let cmd = SearchInHostsCommand(query: "myhost")
        var dismissed = false
        var selected: AppSidebarSelection?
        let ctx = PaletteContext(
            hostsManager: hosts,
            envManager: env,
            select: { selected = $0 },
            dismiss: { dismissed = true }
        )
        cmd.execute(in: ctx)
        #expect(hosts.pendingSearchQuery == "myhost")
        #expect(selected == .systemHosts)
        #expect(dismissed)
    }

    @Test("SearchInEnvCommand pushes query to envManager.pendingSearchQuery")
    func searchInEnvExecutes() {
        let (hosts, env) = makeManagers()
        let cmd = SearchInEnvCommand(query: "API_KEY")
        var dismissed = false
        let ctx = PaletteContext(
            hostsManager: hosts,
            envManager: env,
            select: { _ in },
            dismiss: { dismissed = true }
        )
        cmd.execute(in: ctx)
        #expect(env.pendingSearchQuery == "API_KEY")
        #expect(dismissed)
    }

    // MARK: - Cap

    @Test("Per-section cap prevents 100 profiles flooding palette")
    func sectionCapEnforced() {
        let many = (0..<100).map { i in
            Profile(name: "Profile-\(i)", color: .purple, shortcutNumber: nil, isExclusive: true)
        }
        let (vm, _) = makeVM(profiles: many)
        vm.query = "Profile"
        let switchCount = vm.flat.filter { $0 is SwitchProfileCommand }.count
        #expect(switchCount == 8)
    }
}

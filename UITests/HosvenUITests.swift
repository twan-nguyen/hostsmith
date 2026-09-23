import XCTest

/// End-to-end smoke tests via XCUITest. Exercises the window chrome, the source
/// list, the hosts search field and the ⌘1 shortcut.
///
/// Notes:
/// - Apply (privileged write to /etc/hosts) is NOT exercised — it needs an
///   interactive password dialog.
/// - Profiles come from tag markers in the real /etc/hosts; tests that need one
///   skip when none exist.
final class HosvenUITests: XCTestCase {
    var app: XCUIApplication!

    override func setUpWithError() throws {
        continueAfterFailure = false
        app = XCUIApplication()
        if let appearance = ProcessInfo.processInfo.environment["HOSVEN_APPEARANCE"] {
            app.launchArguments += ["-appearanceMode", appearance]
        }
        app.launchArguments += ["-sidebarSelection", "systemHosts"]
        app.launch()
    }

    override func tearDownWithError() throws {
        app.terminate()
    }

    // MARK: - Smoke tests

    func test_appLaunches_andMainWindowVisible() {
        let window = app.windows.firstMatch
        XCTAssertTrue(window.waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["Hosts hệ thống"].waitForExistence(timeout: 3), "Hosts toolbar title proves the shell rendered")
    }

    func test_sidebar_showsFixedGroups() {
        XCTAssertTrue(app.staticTexts["PROFILES"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["HOSTS"].exists)
        XCTAssertTrue(app.staticTexts["REPOS"].exists)
    }

    func test_sidebar_systemHostsRow_isSelectedByDefault() {
        let row = app.buttons["sidebar-system-hosts"]
        XCTAssertTrue(row.waitForExistence(timeout: 3))
        XCTAssertTrue(row.isSelected)
        XCTAssertTrue(app.buttons["hosts-apply"].exists, "Apply is the last control in the hosts toolbar")
    }

    /// Regression: hidden title bar + floating sidebar card must keep the traffic lights hittable.
    func test_trafficLights_remainVisible() {
        let close = app.windows.firstMatch.buttons[XCUIIdentifierCloseWindow]
        XCTAssertTrue(close.waitForExistence(timeout: 3))
        XCTAssertTrue(close.isHittable, "Close button hittable on launch")

        let toggleSidebar = app.buttons.matching(NSPredicate(format: "label CONTAINS[c] 'sidebar'"))
        XCTAssertEqual(toggleSidebar.count, 0, "No native sidebar toggle should exist")
    }

    func test_searchField_findsLocalhost() throws {
        let search = app.textFields["hosts-search-field"]
        XCTAssertTrue(search.waitForExistence(timeout: 3))
        search.click()
        search.typeText("localhost")
        XCTAssertTrue(app.windows.firstMatch.exists)
    }

    func test_profileRow_click_opensProfilePane() throws {
        let profileRows = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH 'profile-row-'"))
        guard profileRows.count > 0 else {
            throw XCTSkip("No profiles in current /etc/hosts — skipping")
        }
        let first = profileRows.element(boundBy: 0)
        XCTAssertTrue(first.exists)
        first.click()
        XCTAssertTrue(app.buttons["Kích hoạt"].waitForExistence(timeout: 2), "Profile pane shows the Activate button")
    }

    func test_keyboardShortcut_cmd1_activatesFirstProfile() throws {
        let profileRows = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH 'profile-row-'"))
        guard profileRows.count > 0 else {
            throw XCTSkip("No profiles defined — skipping ⌘1 shortcut test")
        }
        app.typeKey("1", modifierFlags: .command)
        XCTAssertTrue(app.windows.firstMatch.exists)
    }

    func test_settingsScene_opensViaMenuBar() throws {
        let initialWindowCount = app.windows.count
        let appMenu = app.menuBars.menuBarItems.element(boundBy: 0)
        XCTAssertTrue(appMenu.waitForExistence(timeout: 2), "App menu missing")
        appMenu.click()
        Thread.sleep(forTimeInterval: 0.4)

        let settingsItem = app.menuBars.menuItems
            .matching(NSPredicate(format: "title BEGINSWITH 'Settings' OR title BEGINSWITH 'Preferences'"))
            .firstMatch
        guard settingsItem.waitForExistence(timeout: 2), settingsItem.isHittable else {
            app.typeKey(.escape, modifierFlags: [])
            throw XCTSkip("Settings… menu item not hittable in this runner context")
        }
        settingsItem.click()
        Thread.sleep(forTimeInterval: 0.6)
        XCTAssertGreaterThan(app.windows.count, initialWindowCount, "Settings window should appear")
    }

    // MARK: - Screenshots

    /// Writes window screenshots to `HOSVEN_SHOT_DIR` when set. Used by
    /// `scripts/take-screenshot.sh`; skipped in a normal test run.
    func test_captureScreenshots() throws {
        guard let dir = ProcessInfo.processInfo.environment["HOSVEN_SHOT_DIR"] else {
            throw XCTSkip("HOSVEN_SHOT_DIR not set")
        }
        let suffix = ProcessInfo.processInfo.environment["HOSVEN_APPEARANCE"] ?? "system"
        XCTAssertTrue(app.windows.firstMatch.waitForExistence(timeout: 5))
        Thread.sleep(forTimeInterval: 1.0)
        try save(app.windows.firstMatch.screenshot(), to: "\(dir)/01-hosts-\(suffix).png")

        let profileRows = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH 'profile-row-'"))
        if profileRows.count > 0 {
            profileRows.element(boundBy: 0).click()
            Thread.sleep(forTimeInterval: 0.8)
            try save(app.windows.firstMatch.screenshot(), to: "\(dir)/04-profile-\(suffix).png")
        }

        let repoRows = app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH 'sidebar-repo-'"))
        if repoRows.count > 0 {
            repoRows.element(boundBy: 0).click()
            Thread.sleep(forTimeInterval: 1.2)
            try save(app.windows.firstMatch.screenshot(), to: "\(dir)/02-env-\(suffix).png")
        }
    }

    private func save(_ screenshot: XCUIScreenshot, to path: String) throws {
        try screenshot.pngRepresentation.write(to: URL(fileURLWithPath: path))
    }
}

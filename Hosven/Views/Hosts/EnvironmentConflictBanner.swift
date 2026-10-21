import SwiftUI

/// Warns when more than one environment has entries on, which happens after a hand edit
/// or with files written by older versions. Each action keeps one environment and writes.
struct EnvironmentConflictBanner: View {
    let profileIds: [UUID]

    @Environment(HostsFileManager.self) private var hostsManager

    private var names: [(id: UUID, name: String)] {
        profileIds.compactMap { id in hostsManager.profiles.first { $0.id == id }.map { (id, $0.name) } }
    }

    var body: some View {
        DSBanner(
            level: .warning,
            title: "\(names.count) môi trường đang cùng bật: \(names.map(\.name).joined(separator: ", "))",
            detail: "Hostname trùng nhau sẽ trỏ theo dòng đứng trước trong /etc/hosts. Giữ một môi trường để tắt các môi trường còn lại.",
            actions: names.map { item in
                .init(label: "Chỉ giữ \(item.name)") {
                    hostsManager.switchEnvironment(to: item.id, policy: .askIfPending)
                }
            }
        )
    }
}

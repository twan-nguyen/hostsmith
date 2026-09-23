import SwiftUI

enum EntryFormMode: Identifiable {
    case add
    case edit(HostEntry)

    var id: String {
        switch self {
        case .add: return "add"
        case .edit(let entry): return entry.id.uuidString
        }
    }
}

struct EntryFormSheet: View {
    let hostsManager: HostsFileManager
    let mode: EntryFormMode
    /// Block preselected for a new entry, e.g. the profile the sheet was opened from.
    var initialTag: String? = nil
    @Environment(\.dismiss) private var dismiss

    @State private var ip = ""
    @State private var hostname = ""
    @State private var comment = ""
    @State private var selectedTag: String = ""
    @State private var ipReason: String?
    @State private var hostnameReason: String?

    private var isEditing: Bool {
        if case .edit = mode { return true }
        return false
    }

    var body: some View {
        DSSheetContainer(
            title: isEditing ? "Sửa entry" : "Entry mới",
            bodyContent: { formBody },
            footer: {
                Button("Huỷ") { dismiss() }
                    .buttonStyle(.ds(.bordered, size: .lg))
                    .keyboardShortcut(.escape)
                Button(isEditing ? "Lưu" : "Thêm entry") { save() }
                    .buttonStyle(.ds(.primary, size: .lg))
                    .keyboardShortcut(.return)
            }
        )
        .frame(width: 460)
        .onAppear {
            switch mode {
            case .edit(let entry):
                ip = entry.ip
                hostname = entry.hostname
                comment = entry.comment
                selectedTag = entry.tag ?? ""
            case .add:
                selectedTag = initialTag ?? ""
                if ip.isEmpty { ip = "127.0.0.1" }
            }
        }
    }

    private var formBody: some View {
        VStack(alignment: .leading, spacing: DS.Spacing.space6) {
            DSField("Địa chỉ IP", text: $ip, prompt: "Địa chỉ IP", monospaced: true, autocorrect: false, invalidReason: ipReason) {
                HStack(spacing: DS.Spacing.space3) {
                    ForEach(["127.0.0.1", "0.0.0.0", "::1"], id: \.self) { quick in
                        Button(quick) { ip = quick }
                            .buttonStyle(.ds(.bordered, size: .sm))
                            .font(DS.Text.monoCaption)
                    }
                }
            }

            DSField("Hostname", text: $hostname, prompt: "Hostname", monospaced: true, autocorrect: false, invalidReason: hostnameReason)

            DSField("Ghi chú", text: $comment, prompt: "Ghi chú")

            if !blockNames.isEmpty {
                VStack(alignment: .leading, spacing: DS.Spacing.space3) {
                    Text("Block")
                        .font(DS.Text.textMenuLabel)
                        .foregroundStyle(DS.Colors.inkSecondary)
                    Picker("", selection: $selectedTag) {
                        Text("Không thuộc block").tag("")
                        ForEach(blockNames, id: \.self) { name in
                            Text(name).tag(name)
                        }
                    }
                    .labelsHidden()
                    .pickerStyle(.menu)
                }
            }
        }
    }

    /// Every block a new entry can join: existing tags plus profiles that have no entries yet.
    private var blockNames: [String] {
        let names = Set(hostsManager.tags.map(\.name) + hostsManager.profiles.map(\.name) + [initialTag ?? ""])
        return names.filter { !$0.isEmpty }.sorted { $0.localizedCaseInsensitiveCompare($1) == .orderedAscending }
    }

    private func save() {
        ipReason = nil
        hostnameReason = nil
        let trimmedIP = ip.trimmingCharacters(in: .whitespaces)
        let trimmedHostname = hostname.trimmingCharacters(in: .whitespaces)

        if trimmedIP.isEmpty { ipReason = "Cần một địa chỉ IP" }
        if trimmedHostname.isEmpty { hostnameReason = "Cần một hostname" }
        if case .add = mode, hostsManager.hostnameExists(trimmedHostname) {
            hostnameReason = "Hostname \(trimmedHostname) đã có trong /etc/hosts"
        }
        guard ipReason == nil, hostnameReason == nil else { return }

        let tagValue = selectedTag.isEmpty ? nil : selectedTag
        switch mode {
        case .add:
            hostsManager.addEntry(ip: trimmedIP, hostname: trimmedHostname, comment: comment.trimmingCharacters(in: .whitespaces), tag: tagValue)
        case .edit(let entry):
            hostsManager.updateEntry(id: entry.id, ip: trimmedIP, hostname: trimmedHostname, comment: comment.trimmingCharacters(in: .whitespaces), tag: tagValue)
        }
        dismiss()
    }
}

import SwiftUI

enum EnvProfileSheetMode: Identifiable {
    case save(repoId: UUID)
    case manage(repoId: UUID)

    var id: String {
        switch self {
        case .save(let id): return "save-\(id.uuidString)"
        case .manage(let id): return "manage-\(id.uuidString)"
        }
    }

    var repoId: UUID {
        switch self {
        case .save(let id), .manage(let id): return id
        }
    }
}

struct EnvProfileSheet: View {
    @Environment(EnvFileManager.self) private var envManager
    let mode: EnvProfileSheetMode
    @Environment(\.dismiss) private var dismiss

    @State private var newProfileName = ""
    @State private var nameReason: String?
    @State private var renamingId: UUID?
    @State private var renameBuffer = ""

    private var repo: EnvRepo? { envManager.repos.first { $0.id == mode.repoId } }

    var body: some View {
        switch mode {
        case .save: saveModeBody
        case .manage: manageModeBody
        }
    }

    // MARK: - Save

    private var saveModeBody: some View {
        DSSheetContainer(
            title: "Lưu trạng thái thành profile",
            subtitle: "Hosven chụp lại nội dung các file .env trong repo này để khôi phục sau.",
            bodyContent: {
                DSField("Tên profile", text: $newProfileName, prompt: "Tên profile", autocorrect: false, invalidReason: nameReason)
            },
            footer: {
                Button("Huỷ") { dismiss() }
                    .buttonStyle(.ds(.bordered, size: .lg))
                    .keyboardShortcut(.escape)
                Button("Lưu profile") { saveProfile() }
                    .buttonStyle(.ds(.primary, size: .lg))
                    .keyboardShortcut(.return)
                    .disabled(newProfileName.trimmingCharacters(in: .whitespaces).isEmpty)
            }
        )
        .frame(width: 460)
    }

    private func saveProfile() {
        nameReason = nil
        do {
            _ = try envManager.saveCurrentAsProfile(repoId: mode.repoId, profileName: newProfileName)
            dismiss()
        } catch {
            nameReason = error.localizedDescription
        }
    }

    // MARK: - Manage

    private var manageModeBody: some View {
        DSSheetContainer(
            title: "Profile của repo",
            bodyContent: {
                Group {
                    if let repo, !repo.profiles.isEmpty {
                        VStack(spacing: 0) {
                            ForEach(Array(repo.profiles.enumerated()), id: \.element.id) { index, profile in
                                DSListRow(
                                    glyph: "doc.text",
                                    title: profile.name,
                                    sublabel: "\(profile.files.count) file · \(profile.capturedAt.formatted(date: .abbreviated, time: .shortened))",
                                    showsHairline: index < repo.profiles.count - 1
                                ) {
                                    Button("Đổi tên") {
                                        renamingId = profile.id
                                        renameBuffer = profile.name
                                    }
                                    .buttonStyle(.ds(.plain, size: .sm))
                                    Button("Xoá") {
                                        envManager.deleteProfile(repoId: mode.repoId, profileId: profile.id)
                                    }
                                    .buttonStyle(.ds(.danger, size: .sm))
                                }
                            }
                        }
                        .dsContentCard(radius: DS.Radius.md)
                        .frame(minHeight: 200, alignment: .top)
                    } else {
                        DSEmptyState(icon: "doc.text", title: "Chưa có profile", message: "Lưu trạng thái hiện tại của repo để tạo profile đầu tiên.")
                            .frame(minHeight: 200)
                    }
                }
            },
            footer: {
                Button("Đóng") { dismiss() }
                    .buttonStyle(.ds(.primary, size: .lg))
                    .keyboardShortcut(.escape)
            }
        )
        .frame(width: 560)
        .alert("Đổi tên profile", isPresented: Binding(get: { renamingId != nil }, set: { if !$0 { renamingId = nil } })) {
            TextField("Tên profile", text: $renameBuffer)
            Button("Đổi tên") { performRename() }
            Button("Huỷ", role: .cancel) { renamingId = nil }
        }
    }

    private func performRename() {
        guard let id = renamingId else { return }
        try? envManager.renameProfile(repoId: mode.repoId, profileId: id, newName: renameBuffer)
        renamingId = nil
        renameBuffer = ""
    }
}

import SwiftUI

enum EnvKeyFormMode: Identifiable {
    case add
    case edit(EnvEntry)

    var id: String {
        switch self {
        case .add: return "add"
        case .edit(let e): return "edit-\(e.id.uuidString)"
        }
    }
}

struct EnvKeyFormSheet: View {
    let mode: EnvKeyFormMode
    let onSave: (String, String, String) -> Void

    @Environment(\.dismiss) private var dismiss
    @State private var key = ""
    @State private var value = ""
    @State private var comment = ""
    @State private var keyReason: String?

    private var isEditing: Bool {
        if case .edit = mode { return true }
        return false
    }

    var body: some View {
        DSSheetContainer(
            title: isEditing ? "Sửa biến" : "Biến mới",
            bodyContent: {
                VStack(alignment: .leading, spacing: DS.Spacing.space6) {
                    DSField("Khoá", text: $key, prompt: "Tên biến", monospaced: true, autocorrect: false, invalidReason: keyReason)
                    DSField("Giá trị", text: $value, prompt: "Giá trị", monospaced: true, autocorrect: false)
                    DSField("Ghi chú", text: $comment, prompt: "Ghi chú")
                }
            },
            footer: {
                Button("Huỷ") { dismiss() }
                    .buttonStyle(.ds(.bordered, size: .lg))
                    .keyboardShortcut(.escape)
                Button(isEditing ? "Lưu" : "Thêm biến") { save() }
                    .buttonStyle(.ds(.primary, size: .lg))
                    .keyboardShortcut(.return)
            }
        )
        .frame(width: 460)
        .onAppear {
            if case .edit(let entry) = mode {
                key = entry.key
                value = entry.value
                comment = entry.comment
            }
        }
    }

    private func save() {
        keyReason = nil
        let trimmedKey = key.trimmingCharacters(in: .whitespaces)
        guard !trimmedKey.isEmpty else {
            keyReason = "Cần một tên biến"
            return
        }
        guard isValidKey(trimmedKey) else {
            keyReason = "Tên biến chỉ gồm A-Z, 0-9 và _, không bắt đầu bằng số"
            return
        }
        onSave(trimmedKey, value, comment.trimmingCharacters(in: .whitespaces))
        dismiss()
    }

    private func isValidKey(_ s: String) -> Bool {
        guard let first = s.first, first.isLetter || first == "_" else { return false }
        return s.allSatisfy { $0.isLetter || $0.isNumber || $0 == "_" }
    }
}

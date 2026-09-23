import SwiftUI

struct ImportSheet: View {
    let hostsManager: HostsFileManager
    @Environment(\.dismiss) private var dismiss
    @State private var importText = ""

    var body: some View {
        DSSheetContainer(
            title: "Nhập entry",
            subtitle: "Dán nội dung dạng hosts. Chỉ những hostname chưa có mới được thêm.",
            bodyContent: {
                CodeEditor(text: $importText, language: .hosts)
                    .frame(minHeight: 240)
                    .dsContentCard()
            },
            footer: {
                Button("Huỷ") { dismiss() }
                    .buttonStyle(.ds(.bordered, size: .lg))
                    .keyboardShortcut(.escape)
                Button("Nhập entry") {
                    hostsManager.importEntries(from: importText)
                    dismiss()
                }
                .buttonStyle(.ds(.primary, size: .lg))
                .disabled(importText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
        )
        .frame(width: 560)
    }
}

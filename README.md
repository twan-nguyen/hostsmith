<p align="center">
  <img src="design/app-icon-1024.png" width="180" alt="Hosven icon"/>
</p>

<h1 align="center">Hosven</h1>

<p align="center">
  Ứng dụng macOS quản lý <code>/etc/hosts</code>, file <code>.env</code> và các cổng đang chạy cho developer.
</p>

## Tính năng

- **Môi trường và block** trong `/etc/hosts`. Mỗi profile là một block `## [tag:Tên]`. Bật một môi trường sẽ tắt các môi trường khác và ghi file trong cùng một lần; block thì bật tắt độc lập. ⌘1–⌘9 đổi môi trường, ⌘0 tắt.
- **Ghi an toàn.** Chỉ hỏi mật khẩu quản trị một lần trong vài phút, tự xoá cache DNS, rồi kiểm tra các hostname vừa ghi đã phân giải đúng chưa. Khi `/etc/hosts` bị công cụ khác sửa, Hosven báo và chờ bạn tải lại thay vì ghi đè.
- **File `.env` theo repo.** Bảng biến có che giá trị bí mật, gộp các khoá trùng thành biến thể, báo biến còn thiếu so với `.env.example`, cảnh báo khi file bị git theo dõi. Lưu và khôi phục cả bộ biến thành profile.
- **Ports.** Liệt kê các cổng TCP đang mở, cho biết tiến trình thuộc repo nào hay container nào, dừng bằng SIGTERM rồi SIGKILL nếu cần.
- **Menu bar** để đổi môi trường, bật nhanh block và xem cổng đang dùng mà không cần mở cửa sổ.
- **⌘K** tìm profile, repo và lệnh.
- **URL scheme** `hosven://` cho script và launcher:
  - `hosven://activate/<profile>`, `hosven://deactivate`
  - `hosven://env/<repo>/profile/<tên>`
  - `hosven://open/<repo>`, `hosven://ports`

## Cài đặt

```bash
brew install --cask twan-nguyen/tap/hosven
```

Cập nhật: `brew upgrade --cask hosven`.

App được ký ad-hoc và không notarize. Cask tự gỡ cờ quarantine; nếu tải file zip bằng trình duyệt thì chạy `xattr -dr com.apple.quarantine /Applications/Hosven.app`.

Người dùng Hosven 1.x cài từ tap cũ `twan-nguyen/hosven` nhận bản 2.0 qua "Kiểm tra cập nhật". Dữ liệu cũ được giữ nguyên: profile, repo và các profile env.

## Dữ liệu

Hosven lưu cấu hình dạng văn bản trong `$XDG_CONFIG_HOME/hosven` hoặc `~/.config/hosven`, nên có thể đưa vào dotfiles:

- `profiles.json`: màu, phím tắt và loại của từng profile.
- `env.json`: danh sách repo và các profile env đã lưu.
- `settings.json`: giao diện, cỡ chữ, menu bar, độ trong suốt.

## Build từ source

Yêu cầu: macOS 15+, Rust (stable), Node 22 và pnpm.

```bash
pnpm install
pnpm tauri dev
```

Kiểm thử: `pnpm test` và `cd src-tauri && cargo test`. Đường dẫn `hosven://` chỉ hoạt động với app đã cài trong `/Applications`, không chạy với `pnpm tauri dev`.

Build bản phát hành: xem [PUBLISHING.md](PUBLISHING.md).

## Giấy phép

MIT

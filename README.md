<p align="center">
  <img src="design/app-icon-1024.png" width="180" alt="Hosven icon"/>
</p>

<h1 align="center">Hosven</h1>

<p align="center">
  Ứng dụng native macOS quản lý <code>/etc/hosts</code> và <code>.env</code> files cho developer.
</p>

<p align="center">
  <img src="design/screenshots/01-hosts.png" width="720" alt="Hosven screenshot"/>
</p>

## Tính năng

- Quản lý `/etc/hosts` theo **môi trường** (Local / Staging / Production): bật một môi trường sẽ tắt các môi trường khác và ghi file ngay; các block còn lại bật tắt độc lập
- Trình soạn thảo có gutter và tô màu cú pháp, áp dụng một lần (quyền quản trị hỏi một lần mỗi phiên, tự flush DNS)
- Quản lý nhiều `.env` file theo repo: bảng, raw, báo biến thiếu so với `.env.example`, ẩn giá trị bí mật, snapshot/restore bộ biến
- Khoá định nghĩa nhiều lần trong một `.env` gộp thành một hàng có menu chọn giá trị đang dùng
- Cảnh báo khi `.env` đang được git theo dõi hoặc chưa có trong `.gitignore`, thêm vào `.gitignore` một nhấn
- Sau khi ghi `/etc/hosts`, kiểm tra lại các hostname vừa đổi bằng resolver hệ thống và báo kết quả
- Ports: liệt kê cổng đang bị chiếm, gắn với repo, dừng tiến trình có xác nhận
- Mở repo, thư mục của tiến trình hoặc file trong Terminal (iTerm, Ghostty, Warp, Terminal) và editor (VS Code, Cursor, Zed, VSCodium) đã cài
- URL scheme `hosven://` để điều khiển từ Raycast, script hay Makefile
- Menu bar panel: đổi môi trường và bật tắt block không cần mở cửa sổ
- **⌘K Command Palette**, giao diện sáng và tối, ⌘1–9 chuyển môi trường nhanh

## Cài đặt

```bash
brew tap twan-nguyen/hosven
brew install hosven
```

Cập nhật: `brew upgrade hosven`

## Sử dụng

1. Mở app, nội dung `/etc/hosts` được tải tự động vào trình soạn thảo
2. Chọn môi trường ở thanh bên và nhấn Kích hoạt: Hosven tắt các môi trường khác và ghi `/etc/hosts` luôn. ⌘1–9 chuyển nhanh, ⌘0 tắt môi trường đang bật. Nếu đang có thay đổi chưa áp dụng, Hosven hỏi trước khi ghi cùng. Đánh dấu một profile là môi trường hay block trong mục Sửa của nó
3. Sửa trực tiếp trong trình soạn thảo hoặc bật tắt block, rồi ⌘S để áp dụng (nhập mật khẩu quản trị lần đầu)
4. ⌘K mở Command Palette, ⌘, mở Cài đặt

### Điều khiển từ ngoài

```bash
open "hosven://activate/Prod-local"          # chuyển sang môi trường hosts và ghi file
open "hosven://deactivate"                   # tắt môi trường đang bật và ghi file
open "hosven://env/end-user/profile/staging" # áp dụng env profile đã lưu cho repo
open "hosven://open/end-user"                # mở repo trong app
open "hosven://ports"                        # mở pane Ports
```

Tên profile và repo không phân biệt hoa thường; tên có khoảng trắng thì mã hoá URL. `activate` và `deactivate` không ghi gì khi trong app đang có thay đổi `/etc/hosts` chưa áp dụng, và báo lỗi trong app.

### Cấu hình

Profile hosts và danh sách repo nằm ở `~/.config/hosven/` (`profiles.json`, `env.json`), hoặc `$XDG_CONFIG_HOME/hosven/` nếu biến đó được đặt, để bạn đưa vào dotfiles. Lần chạy đầu sau khi nâng cấp, Hosven chuyển dữ liệu cũ từ UserDefaults và Application Support sang đó.

## Build từ source

Yêu cầu: macOS 15+, Xcode 15+, [XcodeGen](https://github.com/yonaskolb/XcodeGen).

```bash
make build && make install
```

## Giấy phép

MIT

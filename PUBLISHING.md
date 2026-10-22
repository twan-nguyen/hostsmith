# Phát hành HostSmith

Ghi chú vận hành cho chủ repo. Người dùng cài app bằng Homebrew:

```bash
brew install --cask twan-nguyen/tap/hostsmith
```

## Phát hành bản mới

1. Merge mọi thay đổi vào `master` và push.
2. Tạo tag phiên bản rồi push tag:

   ```bash
   git tag v2.0.1 && git push origin v2.0.1
   ```

3. Theo dõi CI. Bản build universal mất khoảng 10–15 phút:

   ```bash
   gh run watch --repo twan-nguyen/hostsmith
   ```

4. Kiểm tra cask đã trỏ sang bản mới, rồi cập nhật trên máy:

   ```bash
   gh api repos/twan-nguyen/homebrew-tap/contents/Casks/hostsmith.rb --jq .content | base64 -d | grep 'version "'
   brew update && brew upgrade --cask hostsmith
   ```

Workflow [release.yml](.github/workflows/release.yml) làm lần lượt các bước sau:

- Chạy `pnpm test` và `cargo test`.
- Build `HostSmith.app` universal, chạy được trên cả Apple Silicon lẫn Intel. App được ký ad-hoc.
- Nén app thành `HostSmith-X.Y.Z.zip` và tạo GitHub Release.
- Ghi lại `Casks/hostsmith.rb` trong [homebrew-tap](https://github.com/twan-nguyen/homebrew-tap) với phiên bản và `sha256` mới.
- Gửi thông báo trạng thái build qua Telegram.

### Quy tắc

- Tag phải có dạng `vX.Y.Z`. Tag sai dạng làm CI dừng ngay.
- Số phiên bản lấy từ tag, nên không cần sửa `version` trong `tauri.conf.json`, `Cargo.toml` hay `package.json`.
- App không có chữ ký Developer ID và không được notarize. Cask tự gỡ cờ quarantine sau khi cài, nên Gatekeeper không chặn. Nếu tải zip bằng trình duyệt thì phải gỡ tay: `xattr -dr com.apple.quarantine /Applications/HostSmith.app`.
- App không tự cập nhật. Mục "Kiểm tra cập nhật" mở trang Releases; cập nhật bằng `brew upgrade --cask hostsmith`.

### Khi phát hành hỏng

Xem log bằng `gh run view --log-failed --repo twan-nguyen/hostsmith`. Sửa lỗi, rồi xoá release và tag cũ trước khi gắn lại:

```bash
gh release delete v2.0.1 --cleanup-tag --yes --repo twan-nguyen/hostsmith
git tag -d v2.0.1
git tag v2.0.1 && git push origin v2.0.1
```

## Secret của repo

| Secret | Dùng cho |
|---|---|
| `TAP_DEPLOY_KEY` | Đẩy cask lên `twan-nguyen/homebrew-tap` |
| `TELEGRAM_BOT_TOKEN`, `TELEGRAM_CHAT_ID` | Thông báo trạng thái build. Thiếu thì bước này tự bỏ qua |

### Deploy key cho tap

CI đẩy cask lên tap bằng một deploy key có quyền ghi, riêng cho HostSmith. Thu hồi khoá này không ảnh hưởng tới ClusterSmith hay Twanotch.

```bash
ssh-keygen -t ed25519 -N "" -C "hostsmith release" -f ~/hostsmith-tap-key
gh repo deploy-key add ~/hostsmith-tap-key.pub --repo twan-nguyen/homebrew-tap --allow-write --title "hostsmith release"
gh secret set TAP_DEPLOY_KEY --repo twan-nguyen/hostsmith < ~/hostsmith-tap-key
rm ~/hostsmith-tap-key ~/hostsmith-tap-key.pub
```

Khoá không lưu lại trên máy nào. Khi mất khoá, xoá deploy key cũ trong Settings > Deploy keys của `homebrew-tap` rồi tạo lại bằng các lệnh trên.

## Từ Hosven sang HostSmith

Bản 1.x tên Hosven, viết bằng SwiftUI, phát hành qua tap `twan-nguyen/homebrew-hosven` và tự cập nhật bằng Sparkle. Từ 2.0.0 app tên HostSmith: repo `twan-nguyen/hostsmith`, bundle id `com.hostsmith.desktop`, link `hostsmith://`, cấu hình ở `~/.config/hostsmith`. Lần chạy đầu, app chép `profiles.json` và `settings.json` từ `~/.config/hosven` sang và để nguyên thư mục cũ.

Không có cầu nối Sparkle: bản 1.x chỉ có một người dùng, chuyển sang bằng cách gỡ Hosven rồi `brew install --cask twan-nguyen/tap/hostsmith`. Tap cũ và secret `SPARKLE_PRIVATE_KEY`, `RELEASE_PLEASE_TOKEN`, `TAP_REPO_TOKEN` không còn dùng; archive tap cũ và xoá các secret đó khi tiện.

## Build trên máy, không qua phát hành

```bash
pnpm tauri build --bundles app
```

App nằm ở `src-tauri/target/release/bundle/macos/HostSmith.app`. Muốn build bản universal như CI thì chạy `rustup target add x86_64-apple-darwin` một lần, rồi thêm `--target universal-apple-darwin`.

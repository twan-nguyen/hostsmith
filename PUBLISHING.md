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
   gh run watch --repo twan-nguyen/hosven
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
- Riêng tag `v2.0.0`: ký zip bằng khoá Sparkle cũ và ghi `appcast.xml` lên nhánh `gh-pages`, để người dùng Hosven 1.x nhận bản Tauri qua "Kiểm tra cập nhật". Từ bản sau, app cập nhật qua Homebrew.
- Ghi lại `Casks/hostsmith.rb` trong [homebrew-tap](https://github.com/twan-nguyen/homebrew-tap) với phiên bản và `sha256` mới.
- Gửi thông báo trạng thái build qua Telegram.

### Quy tắc

- Tag phải có dạng `vX.Y.Z`. Tag sai dạng làm CI dừng ngay.
- Số phiên bản lấy từ tag, nên không cần sửa `version` trong `tauri.conf.json`, `Cargo.toml` hay `package.json`.
- App không có chữ ký Developer ID và không được notarize. Cask tự gỡ cờ quarantine sau khi cài, nên Gatekeeper không chặn. Nếu tải zip bằng trình duyệt thì phải gỡ tay: `xattr -dr com.apple.quarantine /Applications/HostSmith.app`.
- `src-tauri/Info.plist` giữ `SUPublicEDKey` của bản Swift. Đừng xoá: Sparkle trong Hosven 1.x từ chối bản cập nhật thiếu khoá này.

### Khi phát hành hỏng

Xem log bằng `gh run view --log-failed --repo twan-nguyen/hosven`. Sửa lỗi, rồi xoá release và tag cũ trước khi gắn lại:

```bash
gh release delete v2.0.1 --cleanup-tag --yes --repo twan-nguyen/hosven
git tag -d v2.0.1
git tag v2.0.1 && git push origin v2.0.1
```

## Secret của repo

| Secret | Dùng cho |
|---|---|
| `TAP_DEPLOY_KEY` | Đẩy cask lên `twan-nguyen/homebrew-tap` |
| `SPARKLE_PRIVATE_KEY` | Ký mục appcast cuối cùng cho người dùng 1.x (chỉ tag `v2.0.0`) |
| `TELEGRAM_BOT_TOKEN`, `TELEGRAM_CHAT_ID` | Thông báo trạng thái build. Thiếu thì bước này tự bỏ qua |

### Deploy key cho tap

CI đẩy cask lên tap bằng một deploy key có quyền ghi, riêng cho HostSmith. Thu hồi khoá này không ảnh hưởng tới ClusterSmith hay Twanotch.

```bash
ssh-keygen -t ed25519 -N "" -C "hostsmith release" -f ~/hostsmith-tap-key
gh repo deploy-key add ~/hostsmith-tap-key.pub --repo twan-nguyen/homebrew-tap --allow-write --title "hostsmith release"
gh secret set TAP_DEPLOY_KEY --repo twan-nguyen/hosven < ~/hostsmith-tap-key
rm ~/hostsmith-tap-key ~/hostsmith-tap-key.pub
```

Khoá không lưu lại trên máy nào. Khi mất khoá, xoá deploy key cũ trong Settings > Deploy keys của `homebrew-tap` rồi tạo lại bằng các lệnh trên.

## Đổi tên từ Hosven sang HostSmith

Từ 2.0.0 app tên HostSmith: bundle id `com.hostsmith.desktop`, link `hostsmith://`, cấu hình ở `~/.config/hostsmith`. Lần chạy đầu, app chép `profiles.json` và `settings.json` từ `~/.config/hosven` sang và để nguyên thư mục cũ. Dữ liệu 1.x trong `~/Library/Preferences/com.twannguyen.hosven.plist` vẫn được đọc để chuyển profile.

Thứ tự làm, vì appcast của 1.x nằm trên GitHub Pages của repo `hosven` và Pages không chuyển hướng khi đổi tên repo:

1. Thử cầu nối Sparkle trên máy có Hosven 1.1.1: Sparkle phải cài được `HostSmith.app` dù bundle id và tên app đều khác bản cũ. Nếu Sparkle từ chối, phát hành một bản 1.2.0 bằng Swift chỉ để báo người dùng cài lại bằng Homebrew.
2. Gắn tag `v2.0.0` khi repo còn tên `hosven`, để `appcast.xml` lên đúng địa chỉ 1.x đang đọc.
3. Chờ người dùng 1.x cập nhật (vài tuần), rồi mới đổi tên repo thành `hostsmith` trong Settings của GitHub. Link repo, release và issue cũ được GitHub tự chuyển hướng; các link trong app (`menu.rs`, `Cargo.toml`) có thể đổi sau.

## Tap cũ `twan-nguyen/homebrew-hosven`

Bản 1.x được phát hành qua tap riêng này. Khi chuyển sang `twan-nguyen/tap`:

1. Thêm vào `Casks/hosven.rb` của tap cũ dòng `deprecate! date: "<ngày phát hành 2.0.0>", because: "đã chuyển sang twan-nguyen/tap", replacement_cask: "twan-nguyen/tap/hostsmith"`.
2. Ba đến sáu tháng sau, xoá cask khỏi tap cũ, thêm `tap_migrations.json` và archive repo (không xoá).

## Build trên máy, không qua phát hành

```bash
pnpm tauri build --bundles app
```

App nằm ở `src-tauri/target/release/bundle/macos/HostSmith.app`. Muốn build bản universal như CI thì chạy `rustup target add x86_64-apple-darwin` một lần, rồi thêm `--target universal-apple-darwin`.

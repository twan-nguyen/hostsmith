# Phát hành Hosven

Ghi chú vận hành cho chủ repo. Người dùng cài app bằng Homebrew:

```bash
brew install --cask twan-nguyen/tap/hosven
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
   gh api repos/twan-nguyen/homebrew-tap/contents/Casks/hosven.rb --jq .content | base64 -d | grep 'version "'
   brew update && brew upgrade --cask hosven
   ```

Workflow [release.yml](.github/workflows/release.yml) làm lần lượt các bước sau:

- Chạy `pnpm test` và `cargo test`.
- Build `Hosven.app` universal, chạy được trên cả Apple Silicon lẫn Intel. App được ký ad-hoc.
- Nén app thành `Hosven-X.Y.Z.zip` và tạo GitHub Release.
- Riêng tag `v2.0.0`: ký zip bằng khoá Sparkle cũ và ghi `appcast.xml` lên nhánh `gh-pages`, để người dùng Hosven 1.x nhận bản Tauri qua "Kiểm tra cập nhật". Từ bản sau, app cập nhật qua Homebrew.
- Ghi lại `Casks/hosven.rb` trong [homebrew-tap](https://github.com/twan-nguyen/homebrew-tap) với phiên bản và `sha256` mới.
- Gửi thông báo trạng thái build qua Telegram.

### Quy tắc

- Tag phải có dạng `vX.Y.Z`. Tag sai dạng làm CI dừng ngay.
- Số phiên bản lấy từ tag, nên không cần sửa `version` trong `tauri.conf.json`, `Cargo.toml` hay `package.json`.
- App không có chữ ký Developer ID và không được notarize. Cask tự gỡ cờ quarantine sau khi cài, nên Gatekeeper không chặn. Nếu tải zip bằng trình duyệt thì phải gỡ tay: `xattr -dr com.apple.quarantine /Applications/Hosven.app`.
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

CI đẩy cask lên tap bằng một deploy key có quyền ghi, riêng cho Hosven. Thu hồi khoá này không ảnh hưởng tới ClusterSmith hay Twanotch.

```bash
ssh-keygen -t ed25519 -N "" -C "hosven release" -f ~/hosven-tap-key
gh repo deploy-key add ~/hosven-tap-key.pub --repo twan-nguyen/homebrew-tap --allow-write --title "hosven release"
gh secret set TAP_DEPLOY_KEY --repo twan-nguyen/hosven < ~/hosven-tap-key
rm ~/hosven-tap-key ~/hosven-tap-key.pub
```

Khoá không lưu lại trên máy nào. Khi mất khoá, xoá deploy key cũ trong Settings > Deploy keys của `homebrew-tap` rồi tạo lại bằng các lệnh trên.

## Tap cũ `twan-nguyen/homebrew-hosven`

Bản 1.x được phát hành qua tap riêng này. Khi chuyển sang `twan-nguyen/tap`:

1. Thêm vào `Casks/hosven.rb` của tap cũ dòng `deprecate! date: "<ngày phát hành 2.0.0>", because: "đã chuyển sang twan-nguyen/tap", replacement_cask: "twan-nguyen/tap/hosven"`.
2. Ba đến sáu tháng sau, xoá cask khỏi tap cũ, thêm `tap_migrations.json` và archive repo (không xoá).

## Build trên máy, không qua phát hành

```bash
pnpm tauri build --bundles app
```

App nằm ở `src-tauri/target/release/bundle/macos/Hosven.app`. Muốn build bản universal như CI thì chạy `rustup target add x86_64-apple-darwin` một lần, rồi thêm `--target universal-apple-darwin`.

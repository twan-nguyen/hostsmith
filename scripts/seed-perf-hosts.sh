#!/usr/bin/env bash
# seed-perf-hosts.sh — sinh N host entries vào /etc/hosts dưới tag PERF_TEST.
# Dùng để benchmark scroll perf trong HostSmith.
#
# Usage:
#   ./scripts/seed-perf-hosts.sh seed [count]   # mặc định count=300
#   ./scripts/seed-perf-hosts.sh clean          # xoá block PERF_TEST
#
# Yêu cầu sudo (ghi /etc/hosts).

set -euo pipefail

HOSTS_FILE="/etc/hosts"
TAG_NAME="PERF_TEST"
TAG_START="## [tag:${TAG_NAME}]"
TAG_END="## [/tag:${TAG_NAME}]"
TMP_FILE="$(mktemp -t hostsmith-perf.XXXXXX)"
trap 'rm -f "$TMP_FILE"' EXIT

cmd="${1:-seed}"
count="${2:-300}"

clean_block() {
    # Xoá mọi dòng giữa TAG_START và TAG_END (kể cả markers).
    sudo awk -v start="$TAG_START" -v end="$TAG_END" '
        $0 == start { skip = 1; next }
        $0 == end   { skip = 0; next }
        skip != 1   { print }
    ' "$HOSTS_FILE" > "$TMP_FILE"
    sudo cp "$TMP_FILE" "$HOSTS_FILE"
}

case "$cmd" in
    seed)
        echo "==> Xoá block PERF_TEST cũ (nếu có)..."
        clean_block

        echo "==> Sinh $count entries..."
        {
            echo ""
            echo "$TAG_START"
            for ((i = 1; i <= count; i++)); do
                # IP 10.x.y.z phân tán; hostname kebab-case có-độ-dài-thực-tế
                octet2=$((i / 256 % 256))
                octet3=$((i / 16 % 256))
                octet4=$((i % 256))
                printf '10.%d.%d.%d perf-test-%03d.local.dev # autogen\n' \
                    "$octet2" "$octet3" "$octet4" "$i"
            done
            echo "$TAG_END"
        } | sudo tee -a "$HOSTS_FILE" > /dev/null

        echo "==> Done. Mở HostSmith → tag PERF_TEST sẽ xuất hiện."
        echo "    Cleanup: $0 clean"
        ;;
    clean)
        echo "==> Xoá block PERF_TEST khỏi $HOSTS_FILE..."
        clean_block
        echo "==> Done."
        ;;
    *)
        echo "Usage: $0 {seed [count] | clean}" >&2
        exit 1
        ;;
esac

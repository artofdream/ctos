#!/usr/bin/env bash
# Narrow-viewport overflow probe for the generated mdBook (FR-14 / NFR-06).
# Requires ./book from docs-build.sh. Not a live-site claim.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if [ ! -f book/index.html ] || [ ! -f book/CNAME ]; then
  echo "docs-mobile-probe: run ./scripts/docs-build.sh first" >&2
  exit 1
fi

if ! grep -q 'name="viewport"' book/index.html; then
  echo "docs-mobile-probe: missing viewport meta in book/index.html" >&2
  exit 1
fi

theme_css="$(find book -name '*theme.css' | head -n 1 || true)"
if [ -z "$theme_css" ]; then
  echo "docs-mobile-probe: theme.css not in book/" >&2
  exit 1
fi
grep -q 'overflow-x: auto' "$theme_css"
grep -q 'min-height: 44px' "$theme_css"

CHROME="${CHROME:-}"
if [ -z "$CHROME" ]; then
  for c in google-chrome google-chrome-stable chromium-browser chromium; do
    if command -v "$c" >/dev/null 2>&1; then
      CHROME="$c"
      break
    fi
  done
fi
if [ -z "$CHROME" ]; then
  echo "docs-mobile-probe: static CSS/HTML checks ok; Chrome missing — layout Unknown" >&2
  exit 0
fi

port=8765
python3 -m http.server "$port" --directory book --bind 127.0.0.1 >/tmp/ctos-docs-mobile-http.log 2>&1 &
http_pid=$!
cleanup() { kill "$http_pid" >/dev/null 2>&1 || true; }
trap cleanup EXIT
sleep 0.4

# Same-origin iframe harness written into the gitignored book/ tree.
cat > book/_mobile-probe.html <<'HTML'
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>PROBE_PENDING</title>
</head>
<body>
<iframe id="p" style="border:0;display:block"></iframe>
<script>
(function () {
  var q = new URLSearchParams(location.search);
  var w = parseInt(q.get("w") || "320", 10);
  var page = q.get("page") || "index.html";
  var iframe = document.getElementById("p");
  iframe.width = String(w);
  iframe.height = "900";
  iframe.src = page + (page.indexOf("?") >= 0 ? "&" : "?") + "mobile-probe=1";
  iframe.onload = function () {
    try {
      var doc = iframe.contentDocument;
      var de = doc.documentElement;
      var body = doc.body;
      var cw = de.clientWidth;
      var sw = Math.max(de.scrollWidth, body ? body.scrollWidth : 0);
      var overflow = sw > cw + 2;
      document.title = (overflow ? "OVERFLOW" : "FIT") + " w=" + w + " client=" + cw + " scroll=" + sw + " page=" + page;
    } catch (e) {
      document.title = "PROBE_ERROR " + e;
    }
  };
})();
</script>
</body>
</html>
HTML

probe_one() {
  local w="$1"
  local page="$2"
  local url="http://127.0.0.1:${port}/_mobile-probe.html?w=${w}&page=${page}"
  local html
  html="$("$CHROME" --headless=new --disable-gpu --no-sandbox --hide-scrollbars \
    --virtual-time-budget=12000 --timeout=20000 --dump-dom "$url" 2>/dev/null || true)"
  local title
  title="$(printf '%s\n' "$html" | sed -n 's/.*<title>\([^<]*\)<\/title>.*/\1/p' | head -n 1)"
  if [ -z "$title" ]; then
    echo "docs-mobile-probe: no title for ${page} @ ${w}px" >&2
    return 1
  fi
  echo "docs-mobile-probe: ${title}"
  case "$title" in
    FIT\ *) return 0 ;;
    OVERFLOW\ *)
      echo "docs-mobile-probe: page-wide overflow on ${page} at ${w}px" >&2
      return 1
      ;;
    *)
      echo "docs-mobile-probe: unexpected title: ${title}" >&2
      return 1
      ;;
  esac
}

fail=0
for w in 320 768 1280; do
  probe_one "$w" "index.html" || fail=1
done
probe_one 320 "framework/honesty-ledger.html" || fail=1
probe_one 320 "website.html" || fail=1

if [ "$fail" -ne 0 ]; then
  echo "docs-mobile-probe: FAIL" >&2
  exit 1
fi
echo "docs-mobile-probe: ok (generator layout only; live URL is a separate row)"

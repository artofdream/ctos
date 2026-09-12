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

# Injected only into the gitignored book/ tree. Measures the *iframe*
# viewport so --dump-dom's own width does not hide a 320px overflow.
probe_js='<script id="ctos-mobile-probe">window.addEventListener("load",function(){setTimeout(function(){var d=document.documentElement,b=document.body,cw=d.clientWidth,sw=Math.max(d.scrollWidth,b&&b.scrollWidth||0);d.setAttribute("data-page-overflow",sw>cw+2?"yes":"no");d.setAttribute("data-page-client",String(cw));d.setAttribute("data-page-scroll",String(sw));},400);});</script>'

for html in \
  book/index.html \
  book/website.html \
  book/framework/honesty-ledger.html
do
  if ! grep -q 'id="ctos-mobile-probe"' "$html"; then
    python3 - "$html" "$probe_js" <<'PY'
import pathlib, sys
path = pathlib.Path(sys.argv[1])
text = path.read_text()
needle = "</body>"
idx = text.rfind(needle)
if idx < 0:
    raise SystemExit(f"no </body> in {path}")
path.write_text(text[:idx] + sys.argv[2] + "\n" + text[idx:])
PY
  fi
done

cat > book/_mobile-probe.html <<'HTML'
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>PROBE_PENDING</title>
</head>
<body style="margin:0">
<iframe id="p" style="border:0;display:block"></iframe>
<script>
(function () {
  var q = new URLSearchParams(location.search);
  var w = parseInt(q.get("w") || "320", 10);
  var page = q.get("page") || "index.html";
  var f = document.getElementById("p");
  f.width = String(w);
  f.height = "900";
  f.src = page;
  function copy() {
    try {
      var d = f.contentDocument && f.contentDocument.documentElement;
      var o = d && d.getAttribute("data-page-overflow");
      if (!o) {
        setTimeout(copy, 250);
        return;
      }
      document.documentElement.setAttribute("data-page-overflow", o);
      document.documentElement.setAttribute("data-page-client", d.getAttribute("data-page-client") || "");
      document.documentElement.setAttribute("data-page-scroll", d.getAttribute("data-page-scroll") || "");
    } catch (e) {
      document.documentElement.setAttribute("data-page-overflow", "error");
    }
  }
  f.onload = function () { setTimeout(copy, 700); };
})();
</script>
</body>
</html>
HTML

port=8765
python3 -m http.server "$port" --directory book --bind 127.0.0.1 >/tmp/ctos-docs-mobile-http.log 2>&1 &
http_pid=$!
profile="$(mktemp -d /tmp/ctos-chrome-XXXXXX)"
cleanup() {
  kill "$http_pid" >/dev/null 2>&1 || true
  rm -rf "$profile"
}
trap cleanup EXIT
sleep 0.3

# Fail jsDelivr fast so mermaid.min.js cannot hold the renderer open.
# Diagrams then stay as source text; table/pre overflow is still measured.
probe_one() {
  local w="$1"
  local page="$2"
  local url="http://127.0.0.1:${port}/_mobile-probe.html?w=${w}&page=${page}"
  local html=""
  local chrome_exit=0
  set +e
  html="$(timeout 20 "$CHROME" --headless=new --disable-gpu --no-sandbox \
    --hide-scrollbars --no-first-run --disable-dev-shm-usage \
    --user-data-dir="$profile" \
    --window-size="900,900" \
    --force-device-scale-factor=1 \
    --virtual-time-budget=8000 \
    --host-resolver-rules="MAP cdn.jsdelivr.net 127.0.0.1" \
    --dump-dom "$url" 2>/tmp/ctos-chrome-err.log)"
  chrome_exit=$?
  set -e
  if [ -z "$html" ]; then
    echo "docs-mobile-probe: empty DOM for ${page} @ ${w}px (chrome exit ${chrome_exit})" >&2
    tail -n 8 /tmp/ctos-chrome-err.log >&2 || true
    return 1
  fi
  local overflow client scroll
  overflow="$(printf '%s\n' "$html" | sed -n 's/.*data-page-overflow="\([^"]*\)".*/\1/p' | head -n 1)"
  client="$(printf '%s\n' "$html" | sed -n 's/.*data-page-client="\([^"]*\)".*/\1/p' | head -n 1)"
  scroll="$(printf '%s\n' "$html" | sed -n 's/.*data-page-scroll="\([^"]*\)".*/\1/p' | head -n 1)"
  if [ -z "$overflow" ] || [ "$overflow" = "PROBE_PENDING" ]; then
    echo "docs-mobile-probe: no data-page-overflow for ${page} @ ${w}px" >&2
    return 1
  fi
  echo "docs-mobile-probe: ${page} @ ${w}px overflow=${overflow} client=${client} scroll=${scroll}"
  if [ "$overflow" = "yes" ]; then
    echo "docs-mobile-probe: page-wide overflow on ${page} at ${w}px" >&2
    return 1
  fi
  if [ "$overflow" != "no" ]; then
    echo "docs-mobile-probe: unexpected overflow flag: ${overflow}" >&2
    return 1
  fi
  # The iframe width is the requested viewport. Reject a probe that measured the outer Chrome window.
  if [ -n "$client" ] && [ "$client" -gt $((w + 20)) ]; then
    echo "docs-mobile-probe: clientWidth ${client} is not the ${w}px iframe" >&2
    return 1
  fi
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

# Session memory — 2026-09-12 (mdBook mobile)

Audit: live `https://ctos.artof.link` HTML already has viewport + sidebar toggle. Custom CSS was only `docs/mermaid.css`. architecture.artof.link bar = wrap nav, 44px taps, pre/table overflow-x, images max-width 100%.

Chrome `--dump-dom` ignores `--window-size` (measured client=500). Probe uses a same-origin iframe at the requested width. Chrome also hangs on jsDelivr mermaid; probe maps that host to 127.0.0.1. Screenshot PNG text in the vision tool looks unspaced on both live and local — that is the reader, not missing spaces in the HTML.

Do not treat this file as the honesty ledger.

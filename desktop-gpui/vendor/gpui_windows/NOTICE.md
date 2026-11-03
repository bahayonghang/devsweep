The files in this directory come from Zed commit `322cd509dcccaee1f6bc44725d270598ae8171ad` under the Apache License, Version 2.0. See `LICENSE-APACHE`.

This tree changes the idle frame pump (with its frame trace and frame stats
files). It also changes `WindowsWindow::zoom` so that it restores an
already-maximized window instead of maximizing it again, matching the macOS
zoom toggle.

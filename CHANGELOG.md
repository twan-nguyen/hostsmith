# Changelog

## 2.0.0 (unreleased)

### ⚠ BREAKING CHANGES

* Hosven is renamed HostSmith and rewritten with Tauri 2, Rust and React. Releases come from `vX.Y.Z` tags and install with `brew install --cask twan-nguyen/tap/hostsmith`; the `twan-nguyen/hosven` tap is deprecated and Sparkle hands 1.x users over once.
* the `.env` editor from 1.x is removed

### Features

* environments exclude each other: switching writes `/etc/hosts` once and turns the other environments off; blocks stay independent
* every hosts write checks the hostnames it changed with the system resolver
* ports pane: listening TCP ports attributed to git projects and containers, stop with SIGTERM then SIGKILL
* an environment switcher at the foot of the sidebar; messages float over the window instead of taking room in the panes
* menu bar panel, ⌘K palette and the `hostsmith://` URL scheme
* configuration lives in `~/.config/hostsmith` (`profiles.json`, `settings.json`), copied from `~/.config/hosven` and migrated from the 1.x preferences

### Bug Fixes

* writing `/etc/hosts` no longer drops extra hostnames on a line, entries of a block whose marker differs in case, or bare-address lines, and disabled-entry comments no longer grow on every save
* a failed copy to `/etc/hosts` is reported instead of shown as success, and the file is only replaced by a complete copy
* the root shell runs with a clean environment and absolute tool paths

## [1.1.0](https://github.com/twan-nguyen/hosven/compare/v1.0.10...v1.1.0) (2026-05-18)


### Features

* open edit modal after duplicating hosts entry or env key ([2e2cf01](https://github.com/twan-nguyen/hosven/commit/2e2cf01cbfc7c2aec0d07935e24de5ab579b9f0d))

## [1.0.10](https://github.com/twan-nguyen/hosven/compare/v1.0.9...v1.0.10) (2026-05-16)


### Bug Fixes

* **ci:** escape changelog content without breaking bold tags, strip commit links ([bd1f781](https://github.com/twan-nguyen/hosven/commit/bd1f7813f269f410523927a19110579f0c76253f))
* **ci:** use printf so notification message respects yaml block scalar ([8e57b08](https://github.com/twan-nguyen/hosven/commit/8e57b08ca774bbf87a9ea3666f297c57671dcc43))

## [1.0.9](https://github.com/twan-nguyen/hosven/compare/v1.0.8...v1.0.9) (2026-05-16)


### Bug Fixes

* test full release pipeline with enhanced telegram notification ([f50a815](https://github.com/twan-nguyen/hosven/commit/f50a81568d1794c9e3b7fdaa25d76c0724c6d765))

## [1.0.8](https://github.com/twan-nguyen/hosven/compare/v1.0.7...v1.0.8) (2026-05-16)


### Bug Fixes

* verify release-please PAT triggers downstream build ([95708e6](https://github.com/twan-nguyen/hosven/commit/95708e6ac279492202206888ea22c14ad2c7cae3))

## [1.0.7](https://github.com/twan-nguyen/hosven/compare/v1.0.6...v1.0.7) (2026-05-16)


### Bug Fixes

* trigger first automated release to verify pipeline ([675cde8](https://github.com/twan-nguyen/hosven/commit/675cde8ac24a5e9b2fbc46a4fc055dba9a2f8f7d))

## v1.0.0 — 2026-05-08

Initial release of Hosven (renamed from Devly).

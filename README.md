# FocusOS

Offline-first study workspace. Tauri 2 + Rust backend, React + TypeScript + Vite frontend.

**Status: Milestone 1 (scaffold).** Not yet built or tested by the author of this archive.

## Prerequisites (Windows)
- Rust (MSVC toolchain): `rustup default stable-msvc`
- Node.js 20+
- Visual Studio Build Tools, "Desktop development with C++"
- WebView2 (preinstalled on Windows 10 1803+ / 11)

## Run
```powershell
npm install
npm run tauri dev
```

## Checks
```powershell
npm run typecheck
cd src-tauri; cargo fmt --check; cargo clippy -- -D warnings; cd ..
```

## Release build
```powershell
npm run tauri build
```

## Notes
- Icons in `src-tauri/icons` are placeholders. Replace with your own: `npx tauri icon path\to\logo.png`.
- Change the bundle identifier `com.example.focusos` in `src-tauri/tauri.conf.json` before release.
- Dependencies use semver ranges; first `npm install` / `cargo build` creates lockfiles. Commit them.

# QuotaTray

QuotaTray is a local-first desktop tray app for monitoring AI provider usage, quota, limits, and reset windows from one compact popover.

Built with **Tauri v2**, **React**, **TypeScript**, and **Rust**.

## Features

- Cross-platform tray/menu bar desktop app
- Native-style responsive popover UI
- Local SQLite storage
- OS keychain storage for API keys
- Manual refresh and background polling
- Provider status states: `ok`, `warning`, `critical`, `unknown`, `error`
- Browser preview mode for UI development
- Real provider support in Tauri desktop runtime

## Providers

Current implementation:

| Provider | Auth | Status |
|---|---|---|
| Codex | Local Codex session from `~/.codex/auth.json` | Implemented |
| OpenAI / GPT | API key from `platform.openai.com` | Implemented validation |
| OpenRouter | API key | Implemented |
| Mock Provider | None | Implemented |

### Codex provider

QuotaTray can reuse an existing local Codex login:

```txt
~/.codex/auth.json
```

It reads the local Codex OAuth token and calls the Codex usage endpoint. If the access token is stale, QuotaTray attempts to refresh it using the stored refresh token.

> Codex quota reading only works in the Tauri desktop app, not in browser preview mode.

### OpenAI / GPT provider

OpenAI support currently validates an API key with OpenAI API access.

Important:

- Use an API key from `https://platform.openai.com/api-keys`.
- ChatGPT Plus/Pro login is not the same as an OpenAI API key.
- ChatGPT web/app quota is not exposed through a stable public API.

## Development

Install dependencies:

```bash
npm install
```

Run frontend-only browser preview:

```bash
npm run dev
```

Run the real desktop app:

```bash
npm run tauri:dev:msys
```

Build frontend:

```bash
npm run build
```

Build/check Rust backend:

```bash
cd src-tauri
cargo check
```

Build desktop package:

```bash
npm run tauri:build
```

## Browser preview vs desktop runtime

`npm run dev` starts Vite in a browser. This mode is only for UI preview and cannot access local files, SQLite, OS keychain, or real provider credentials.

Use this for real provider data:

```bash
npm run tauri:dev:msys
```

If you see this message:

```txt
Browser preview mode
```

then you are not running the Tauri desktop app.

## Windows setup

This project currently uses the Windows GNU Rust toolchain.

Required tools:

- Rust / Cargo
- Node.js / npm
- MSYS2 MinGW-w64 binutils and GCC
- WebView2 runtime

Install MSYS2 packages:

```bash
C:/msys64/usr/bin/bash -lc "pacman -S --noconfirm --needed mingw-w64-x86_64-binutils mingw-w64-x86_64-gcc"
```

Make sure this is available in PATH:

```txt
C:\msys64\mingw64\bin
```

The helper script already injects that path:

```bash
npm run tauri:dev:msys
```

If running manually:

```bash
export PATH="/c/msys64/mingw64/bin:$PATH"
npm run tauri:dev
```

## Project structure

```txt
.
├─ src/                  # React frontend
├─ src-tauri/            # Tauri/Rust backend
├─ docs/                 # Product and UI docs
├─ prd.md                # Product requirements document
├─ package.json
└─ README.md
```

## Scripts

```bash
npm run dev             # Vite browser preview
npm run build           # TypeScript + Vite build
npm run tauri:dev       # Tauri dev app
npm run tauri:dev:msys  # Tauri dev app with MSYS2 PATH on Windows
npm run tauri:build     # Tauri package build
```

## Security & privacy

- Local-first by default
- API keys are stored in the OS keychain
- Codex reads local `~/.codex/auth.json`
- No QuotaTray cloud account
- No default telemetry
- Do not commit real credentials or `.env` files

## Status

Early MVP / prototype.

Primary focus:

1. Stable Codex usage reading
2. OpenAI/OpenRouter API key provider flows
3. Responsive tray popover UI
4. Cross-platform packaging

## License

TBD

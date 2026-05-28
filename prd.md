# PRD — QuotaTray

## 1. Ringkasan Produk

**QuotaTray** adalah aplikasi desktop lintas platform berbasis tray/menu bar untuk memantau penggunaan, kuota, limit, dan waktu reset berbagai layanan AI/coding assistant dari satu tempat.

Aplikasi berjalan di background dan menampilkan status penggunaan AI provider secara ringkas melalui tray icon, popup dashboard, dan notifikasi limit.

Target awal bukan menggantikan dashboard resmi tiap provider, tetapi menyediakan **single glance usage monitor** untuk developer dan AI power user yang memakai banyak layanan seperti OpenRouter, OpenAI, Gemini, Claude, Cursor, GitHub Copilot, dan lainnya.

## 2. Nama Produk

**QuotaTray**

Alternatif nama yang sempat dipertimbangkan:

- TokenPulse
- LimitDock
- AI Usage Tray

Nama final yang direkomendasikan: **QuotaTray** karena jelas, pendek, dan cocok untuk macOS menu bar, Windows system tray, serta Linux tray/AppIndicator.

## 3. Tagline

> Track AI coding limits across your tools, right from your tray.

## 4. Problem Statement

Developer modern sering memakai banyak layanan AI sekaligus, misalnya OpenAI API, OpenRouter, Gemini, Claude, Cursor, dan GitHub Copilot. Setiap layanan memiliki sistem kuota, billing, reset limit, atau rate limit yang berbeda.

Masalah utama:

1. User harus membuka banyak dashboard provider untuk mengecek usage.
2. Beberapa provider tidak menampilkan status limit secara cepat.
3. User sering baru sadar limit habis ketika sedang bekerja.
4. Reset time dan remaining quota tersebar di banyak tempat.
5. Tidak ada indikator global di desktop untuk kondisi penggunaan AI.

QuotaTray menyelesaikan masalah ini dengan menyediakan dashboard kecil di tray yang menampilkan status semua provider secara terpadu.

## 5. Target User

### 5.1 Primary User

**Developer / AI power user** yang:

- Memakai beberapa AI provider untuk coding.
- Menggunakan API key pribadi.
- Sering berpindah antara OpenAI, Gemini, OpenRouter, Claude, Cursor, atau Copilot.
- Ingin tahu apakah quota atau limit hampir habis tanpa membuka banyak dashboard.

### 5.2 Secondary User

- Indie hacker yang memakai banyak model AI.
- Team lead yang ingin memantau usage account pribadi/team.
- Researcher yang sering memakai berbagai LLM provider.

## 6. Goals

### 6.1 Product Goals

1. Menampilkan status usage AI provider dari tray/menu bar.
2. Memberikan indikator cepat jika kuota mendekati habis.
3. Menyediakan popup dashboard sederhana untuk melihat detail provider.
4. Menyediakan sistem provider adapter yang mudah ditambah.
5. Menyimpan credential secara aman menggunakan OS keychain.
6. Berjalan di macOS, Windows, dan Linux.

### 6.2 MVP Goals

MVP harus mampu:

1. Berjalan sebagai tray app di 3 OS.
2. Menampilkan popup dashboard.
3. Menyimpan konfigurasi lokal.
4. Menyimpan secret di OS keychain.
5. Mendukung minimal 1 real provider: **OpenRouter**.
6. Mendukung mock provider untuk testing UI.
7. Melakukan refresh usage manual dan otomatis.
8. Menampilkan usage percentage, last refresh, status, dan error state.

## 7. Non-Goals

Untuk MVP, QuotaTray **tidak** akan:

1. Mendukung semua provider AI sekaligus.
2. Membaca cookie browser secara otomatis.
3. Melakukan scraping dashboard provider.
4. Membypass quota, limit, atau rate limit provider.
5. Menyediakan team billing management.
6. Menyediakan cloud sync.
7. Menjadi replacement penuh untuk dashboard resmi provider.
8. Mengirim API key atau usage data ke server pihak ketiga milik QuotaTray.
9. Menjalankan model lokal.
10. Mengubah konfigurasi akun provider.

## 8. Platform Support

| OS | Bentuk App | Target MVP |
|---|---|---|
| macOS | Menu bar app | Ya |
| Windows | System tray app | Ya |
| Linux | Tray/AppIndicator app | Ya |

Catatan:

- Branding produk harus memakai istilah **tray app**, bukan hanya **menu bar app**, agar cocok untuk semua OS.
- Linux tray behavior bisa berbeda antar desktop environment. MVP cukup mendukung environment umum seperti GNOME dengan AppIndicator, KDE, dan Ubuntu-based desktop.

## 9. Rekomendasi Stack

### 9.1 Desktop App

- **Tauri v2**
- **Rust backend**
- **React + Vite + TypeScript frontend**

### 9.2 UI

- Tailwind CSS
- shadcn/ui
- Lucide Icons
- Recharts untuk chart sederhana jika dibutuhkan

### 9.3 Rust Core

- `reqwest` untuk HTTP client
- `tokio` untuk async runtime/scheduler
- `serde` untuk serialisasi
- `sqlx` atau `rusqlite` untuk SQLite
- `keyring` / `keyring-rs` untuk OS credential storage
- Tauri tray/menu APIs

### 9.4 Storage

- SQLite untuk data non-secret
- OS keychain untuk API key/token
- Config file untuk setting ringan jika diperlukan

### 9.5 Build & Release

- GitHub Actions
- macOS: `.dmg`, `.app`, Homebrew later
- Windows: `.msi` / `.exe`
- Linux: AppImage, `.deb`, `.rpm`

## 10. Arsitektur High-Level

```txt
QuotaTray Desktop App
├─ React UI
│  ├─ Dashboard Popup
│  ├─ Provider Cards
│  ├─ Usage Bars
│  ├─ Reset Countdown
│  ├─ Settings
│  └─ Error / Empty States
│
├─ Tauri Commands
│  ├─ get_providers
│  ├─ add_provider
│  ├─ remove_provider
│  ├─ refresh_provider
│  ├─ refresh_all
│  ├─ get_settings
│  └─ update_settings
│
├─ Rust Core
│  ├─ Provider Adapter Registry
│  ├─ Scheduler Polling
│  ├─ Secure Credential Access
│  ├─ Local SQLite Storage
│  ├─ Usage Normalizer
│  └─ Error Mapper
│
├─ Providers
│  ├─ Mock Provider
│  ├─ OpenRouter Provider
│  ├─ OpenAI Provider later
│  ├─ Gemini Provider later
│  ├─ Claude Provider later
│  ├─ GitHub Copilot later
│  └─ Cursor later
│
└─ System Integration
   ├─ macOS Menu Bar
   ├─ Windows System Tray
   └─ Linux AppIndicator/Tray
```

## 11. Provider Adapter System

Provider harus dibuat modular agar mudah ditambah tanpa mengubah core app.

### 11.1 Konsep Interface

Contoh interface konseptual TypeScript:

```ts
interface Provider {
  id: string;
  name: string;
  authType: "api_key" | "oauth" | "cookie" | "cli" | "local";
  getUsage(): Promise<UsageResult>;
}
```

Implementasi aktual di MVP berada di Rust, tetapi frontend boleh memakai type yang sepadan untuk rendering.

### 11.2 Normalized Usage Result

Semua provider harus dinormalisasi ke format umum:

```ts
type ProviderStatus = "ok" | "warning" | "critical" | "unknown" | "error";

type UsageUnit = "credits" | "usd" | "tokens" | "requests" | "percentage" | "unknown";

interface UsageResult {
  providerId: string;
  providerName: string;
  status: ProviderStatus;
  used?: number;
  limit?: number;
  remaining?: number;
  percentage?: number;
  unit: UsageUnit;
  resetAt?: string;
  lastRefreshAt: string;
  message?: string;
  raw?: unknown;
}
```

### 11.3 Status Mapping

| Condition | Status |
|---|---|
| Usage berhasil dan < 70% | ok |
| Usage >= 70% dan < 90% | warning |
| Usage >= 90% | critical |
| Provider tidak menyediakan limit jelas | unknown |
| Auth/network/API error | error |

Threshold harus bisa diubah dari settings pada versi setelah MVP. Untuk MVP, hardcoded threshold boleh digunakan.

## 12. Provider Scope

### 12.1 Provider Matrix

| Provider | Auth | Usage API | Reset Info | Difficulty | MVP |
|---|---|---:|---:|---:|---:|
| Mock Provider | none | Ya | Ya | Rendah | Ya |
| OpenRouter | API key | Ya | Sebagian | Rendah-Sedang | Ya |
| OpenAI | API key | Tergantung endpoint/account | Sebagian | Sedang | Setelah MVP |
| Gemini | API key/OAuth | Tergantung usage/billing API | Sebagian | Sedang | Setelah MVP |
| Anthropic/Claude | API key/console data | Terbatas | Terbatas | Sedang-Tinggi | Setelah MVP |
| GitHub Copilot | OAuth/CLI/GraphQL possible | Tidak sederhana | Terbatas | Tinggi | Later |
| Cursor | Cookie/OAuth/local | Tidak stabil | Tidak stabil | Tinggi | Later |
| Ollama | local | Local metrics possible | Tidak relevan | Rendah | Optional |

### 12.2 MVP Provider

MVP resmi hanya wajib mendukung:

1. **Mock Provider**
   - Untuk development, demo, dan UI testing.
2. **OpenRouter**
   - Provider real pertama karena relatif mudah via API key.

Provider lain masuk roadmap setelah fondasi stabil.

## 13. Core User Flows

### 13.1 First Launch / Empty State

1. User membuka QuotaTray.
2. App muncul di tray.
3. User klik tray icon.
4. Popup dashboard terbuka.
5. Jika belum ada provider, tampil empty state:
   - “No providers connected yet.”
   - Button: “Add Provider”
   - Button: “Try Mock Provider”

Acceptance criteria:

- App tidak crash meskipun belum ada provider.
- User bisa membuka settings/add provider dari empty state.

### 13.2 Add Provider — OpenRouter

1. User klik “Add Provider”.
2. User memilih OpenRouter.
3. User memasukkan API key.
4. App menyimpan API key ke OS keychain.
5. App melakukan test request.
6. Jika valid, provider ditambahkan ke SQLite.
7. Dashboard menampilkan usage OpenRouter.

Acceptance criteria:

- API key tidak disimpan di SQLite.
- Jika API key invalid, user mendapat error yang jelas.
- Jika network gagal, user mendapat error yang jelas.
- Setelah restart app, provider tetap ada dan secret tetap bisa diakses dari keychain.

### 13.3 Refresh Usage

1. User klik tombol refresh pada provider atau global refresh.
2. App mengambil credential dari keychain.
3. App memanggil endpoint provider.
4. App menormalisasi hasil.
5. UI diperbarui.
6. `lastRefreshAt` diperbarui.

Acceptance criteria:

- Refresh manual bekerja.
- Loading state tampil.
- Error state tampil jika request gagal.
- Hasil terakhir tetap tersimpan walaupun refresh berikutnya gagal.

### 13.4 Automatic Polling

1. App berjalan di background.
2. Scheduler melakukan refresh berkala.
3. Interval default MVP: 15 menit.
4. UI/tray icon diperbarui setelah refresh.

Acceptance criteria:

- Polling tidak berjalan terlalu sering.
- Polling bisa dimatikan dari settings minimal setelah MVP; untuk MVP boleh hardcoded.
- App tetap responsif saat polling.

### 13.5 Remove Provider

1. User membuka provider settings.
2. User klik remove provider.
3. App meminta konfirmasi.
4. App menghapus provider dari SQLite.
5. App menghapus secret dari OS keychain.

Acceptance criteria:

- Provider hilang dari dashboard.
- Secret ikut terhapus.
- App tidak crash jika keychain delete gagal, tetapi error harus tercatat.

## 14. Fitur MVP

### 14.1 Tray Icon

Fungsi:

- Menampilkan status global quota.
- Klik membuka popup dashboard.
- Icon berubah berdasarkan status global.

Status global:

| Kondisi | Tray Status |
|---|---|
| Semua provider ok | Normal |
| Ada provider warning | Warning |
| Ada provider critical | Critical |
| Semua provider unknown | Unknown |
| Ada error auth/network | Error indicator |

Acceptance criteria:

- Tray icon muncul di macOS, Windows, Linux.
- Klik tray membuka dashboard.
- Status berubah setelah refresh.

### 14.2 Popup Dashboard

Komponen:

- Header app name
- Global status summary
- Provider list
- Usage bar per provider
- Last refresh
- Manual refresh button
- Settings button

Acceptance criteria:

- Dashboard bisa dibuka dari tray.
- Minimal menampilkan mock provider dan OpenRouter.
- Responsive untuk ukuran popup kecil.

### 14.3 Provider Card

Setiap provider card menampilkan:

- Provider name
- Status badge
- Usage percentage jika tersedia
- Used / limit / remaining jika tersedia
- Unit
- Reset countdown jika tersedia
- Last refresh
- Error message jika ada
- Refresh button

Acceptance criteria:

- Card tetap berguna walaupun provider tidak memberi limit.
- Error provider tidak merusak provider lain.

### 14.4 Settings

MVP settings minimal:

- Add provider
- Remove provider
- Refresh all
- App version

Post-MVP settings:

- Polling interval
- Warning threshold
- Critical threshold
- Launch at login
- Theme
- Export diagnostics

### 14.5 Local Persistence

MVP harus menyimpan:

- Provider configuration non-secret
- Last usage result
- App settings dasar

Tidak boleh menyimpan:

- API key plaintext di SQLite
- Token di log

## 15. CLI Scope

CLI adalah fitur bagus, tetapi bukan wajib untuk MVP pertama.

### 15.1 CLI Target Setelah MVP

Command potensial:

```bash
quotatray status
quotatray refresh
quotatray providers
quotatray add openrouter
quotatray remove openrouter
```

Untuk PRD MVP, CLI masuk **post-MVP** agar scope tetap kecil.

## 16. Data Model

### 16.1 SQLite Tables

#### `providers`

| Column | Type | Description |
|---|---|---|
| id | TEXT PRIMARY KEY | Internal provider instance id |
| provider_type | TEXT | `openrouter`, `mock`, etc |
| display_name | TEXT | User-facing name |
| auth_type | TEXT | `api_key`, `oauth`, etc |
| keychain_service | TEXT | Keychain service name |
| keychain_account | TEXT | Keychain account/key id |
| enabled | INTEGER | 1/0 |
| created_at | TEXT | ISO timestamp |
| updated_at | TEXT | ISO timestamp |

#### `usage_snapshots`

| Column | Type | Description |
|---|---|---|
| id | TEXT PRIMARY KEY | Snapshot id |
| provider_id | TEXT | FK to providers.id |
| status | TEXT | ok/warning/critical/unknown/error |
| used | REAL NULL | Used amount |
| limit_value | REAL NULL | Limit amount |
| remaining | REAL NULL | Remaining amount |
| percentage | REAL NULL | Usage percentage |
| unit | TEXT | credits/usd/tokens/etc |
| reset_at | TEXT NULL | ISO timestamp |
| message | TEXT NULL | Human-readable note/error |
| raw_json | TEXT NULL | Optional sanitized raw response |
| created_at | TEXT | ISO timestamp |

#### `settings`

| Column | Type | Description |
|---|---|---|
| key | TEXT PRIMARY KEY | Setting key |
| value | TEXT | Setting value as string/json |
| updated_at | TEXT | ISO timestamp |

### 16.2 Keychain Storage

Keychain entry:

```txt
service: com.quotatray.provider.<provider_type>
account: <provider_id>
secret: <api_key_or_token>
```

Rules:

- Secret hanya disimpan di OS keychain.
- SQLite hanya menyimpan pointer ke keychain.
- Secret tidak boleh tampil di logs.
- Secret tidak boleh dikirim ke frontend kecuali benar-benar perlu; idealnya tidak pernah.

## 17. Security Requirements

1. API key wajib disimpan di OS keychain.
2. Jangan menyimpan secret di SQLite/config/log.
3. Jangan print request header authorization di log.
4. Error message harus disanitasi.
5. Frontend tidak boleh menerima API key plaintext setelah disimpan.
6. Semua request provider harus memakai HTTPS.
7. Tidak ada telemetry default pada MVP.
8. Jika diagnostics/export ditambahkan nanti, harus melakukan redaction.
9. App tidak boleh membaca browser cookie pada MVP.
10. App tidak boleh mengirim data usage ke server QuotaTray.

## 18. Privacy Requirements

QuotaTray MVP adalah local-first app.

- Data provider disimpan lokal.
- API key disimpan di OS keychain.
- Usage snapshot disimpan di SQLite lokal.
- Tidak ada akun QuotaTray.
- Tidak ada cloud sync.
- Tidak ada analytics default.

Jika analytics ditambahkan nanti, harus opt-in.

## 19. Error Handling

### 19.1 Error Types

| Error | UI Message |
|---|---|
| Invalid API key | “API key is invalid. Please update your credentials.” |
| Network timeout | “Could not reach provider. Try again later.” |
| Rate limited | “Provider rate limit reached. Refresh later.” |
| Provider unsupported response | “Provider returned an unsupported response.” |
| Keychain unavailable | “Could not access secure credential storage.” |
| Unknown error | “Something went wrong.” |

### 19.2 Behavior

- Error pada satu provider tidak boleh menghentikan refresh provider lain.
- Jika refresh gagal, tampilkan error tetapi pertahankan snapshot terakhir.
- Harus ada timestamp untuk last successful refresh dan last attempted refresh jika memungkinkan.

## 20. UI/UX Direction

### 20.1 Design Principles

- Cepat dibaca dalam 3 detik.
- Tidak terasa seperti dashboard besar.
- Fokus ke status: aman, hampir habis, habis/error.
- Minimal, padat, tapi tetap polished.
- Cocok untuk dark mode.

### 20.2 Visual Status

| Status | Color Direction |
|---|---|
| ok | Green/neutral |
| warning | Amber |
| critical | Red |
| unknown | Gray/blue |
| error | Red/purple |

### 20.3 Popup Layout MVP

```txt
┌──────────────────────────────┐
│ QuotaTray              ↻  ⚙  │
│ All systems ok               │
├──────────────────────────────┤
│ OpenRouter              OK   │
│ ███████░░░ 72%               │
│ $7.20 / $10.00               │
│ Resets: unknown              │
│ Last refresh: 2m ago         │
├──────────────────────────────┤
│ Mock Provider        Warning │
│ ████████░░ 82%              │
│ 820 / 1000 requests          │
│ Resets in: 3h 12m            │
└──────────────────────────────┘
```

## 21. OpenRouter MVP Integration

### 21.1 Auth

- User memasukkan OpenRouter API key.
- API key disimpan di keychain.
- Request memakai header authorization sesuai dokumentasi OpenRouter.

### 21.2 Data yang Diambil

Target data:

- Credit/usage balance jika tersedia.
- Limit atau remaining credits jika tersedia.
- Percentage dihitung jika `used` dan `limit` tersedia.
- Jika hanya balance tersedia, status boleh `unknown` dengan message yang jelas.

### 21.3 Acceptance Criteria

- User bisa menambahkan API key OpenRouter.
- App bisa melakukan validasi dasar dengan request ke OpenRouter.
- App bisa menampilkan data usage/balance yang tersedia.
- Jika limit tidak tersedia, UI tetap menampilkan informasi yang ada tanpa menganggap error.

## 22. Mock Provider

Mock provider diperlukan untuk:

- Development UI tanpa API key.
- Testing status ok/warning/critical/error.
- Demo app.

Mock provider harus bisa menghasilkan:

- Usage 30% ok
- Usage 75% warning
- Usage 95% critical
- Unknown status
- Error state

Untuk MVP, mock mode boleh di-hardcode atau dipilih dari settings/dev flag.

## 23. Scheduler / Polling

### 23.1 MVP Behavior

- Auto refresh interval default: 15 menit.
- Manual refresh selalu tersedia.
- Refresh harus async dan tidak memblokir UI.
- Jika app sleep/wake, app boleh refresh saat aktif kembali.

### 23.2 Post-MVP

- User configurable interval: 5, 15, 30, 60 menit.
- Disable auto refresh.
- Per-provider polling interval.

## 24. Notifications

### 24.1 MVP

Notifikasi desktop tidak wajib untuk v0.1.

### 24.2 Post-MVP

Notifikasi jika:

- Usage melewati warning threshold.
- Usage melewati critical threshold.
- Provider auth gagal.
- Reset sudah terjadi.

## 25. Release Plan

### Phase 0 — Project Setup

Deliverables:

- Tauri v2 app initialized.
- React + Vite + TypeScript setup.
- Tailwind + shadcn/ui configured.
- Basic tray icon muncul.
- Basic popup window terbuka.

Acceptance criteria:

- App bisa dijalankan lokal.
- Tray icon muncul minimal di development OS.
- Popup menampilkan placeholder dashboard.

### Phase 1 — Local Core & Mock Provider

Deliverables:

- SQLite setup.
- Provider registry abstraction.
- Mock provider.
- Provider card UI.
- Manual refresh.

Acceptance criteria:

- User bisa melihat mock usage.
- Status ok/warning/critical/error tampil benar.
- Snapshot tersimpan lokal.

### Phase 2 — Secure Credentials & OpenRouter

Deliverables:

- Keychain integration.
- Add provider flow.
- OpenRouter provider adapter.
- Error handling for invalid key/network.

Acceptance criteria:

- API key OpenRouter tersimpan di keychain.
- OpenRouter usage/balance tampil.
- Restart app tidak menghapus provider.

### Phase 3 — Tray Status & Polling

Deliverables:

- Global status computation.
- Tray icon status update.
- Auto polling 15 menit.
- Refresh all.

Acceptance criteria:

- Tray status berubah sesuai provider status.
- Polling berjalan tanpa freeze UI.

### Phase 4 — Packaging MVP

Deliverables:

- Build macOS.
- Build Windows.
- Build Linux.
- Basic README.
- Known limitations documented.

Acceptance criteria:

- Installer/build artifact tersedia untuk 3 OS.
- App bisa dibuka dan menjalankan flow MVP.

## 26. MVP Acceptance Criteria Summary

MVP dianggap selesai jika:

1. App berjalan sebagai tray app.
2. User bisa membuka popup dashboard dari tray.
3. User bisa menambahkan mock provider.
4. User bisa menambahkan OpenRouter provider dengan API key.
5. API key tersimpan di OS keychain, bukan SQLite.
6. Usage/balance OpenRouter tampil jika API mengembalikan data.
7. Provider card menampilkan loading, success, unknown, dan error state.
8. Manual refresh bekerja.
9. Auto refresh berjalan minimal setiap 15 menit.
10. Snapshot terakhir tersimpan lokal.
11. Tray icon/global status berubah berdasarkan provider status.
12. App tidak mengirim data ke server QuotaTray.
13. Build lokal bisa dibuat minimal untuk satu OS, dengan rencana packaging 3 OS.

## 27. Technical Risks

| Risk | Impact | Mitigation |
|---|---|---|
| Provider tidak punya usage API publik | Data tidak lengkap | Mulai dari provider yang jelas seperti OpenRouter |
| Auth tiap provider beda | Kompleksitas tinggi | Provider adapter modular |
| Keychain behavior beda per OS | Bug lintas platform | Abstraction + manual testing OS |
| Linux tray tidak konsisten | UX berbeda | Dokumentasikan supported desktop env |
| Provider endpoint berubah | Integration rusak | Error handling + adapter tests |
| Scope melebar ke banyak provider | MVP terlambat | Batasi MVP ke Mock + OpenRouter |

## 28. Roadmap Setelah MVP

### v0.2

- OpenAI provider.
- Gemini provider.
- Configurable polling interval.
- Warning/critical threshold settings.
- Desktop notifications.

### v0.3

- Anthropic/Claude provider jika usage API memungkinkan.
- CLI command dasar.
- Better charts/history.
- Launch at login.

### v0.4

- GitHub Copilot investigation.
- Cursor investigation.
- Import/export config without secrets.
- Diagnostics export with redaction.

### v1.0

- Stable provider plugin architecture.
- Signed installers.
- Auto update.
- Full documentation.
- Optional opt-in telemetry.

## 29. Open Questions

1. Apakah QuotaTray akan open source atau closed source?
2. Apakah app perlu auto-update sejak awal?
3. Apakah target distribusi awal GitHub Releases saja?
4. Apakah OpenAI/Gemini harus masuk v0.1 atau cukup v0.2?
5. Apakah perlu mode team/account sharing di masa depan?
6. Apakah nama final tetap QuotaTray?

## 30. Recommended Execution Scope

Untuk mulai coding, scope paling aman adalah:

> **QuotaTray v0.1: Tauri tray app dengan React dashboard, SQLite local storage, OS keychain, mock provider, OpenRouter provider, manual refresh, auto polling 15 menit, dan global tray status.**

Jangan mulai dari banyak provider. Selesaikan fondasi dulu, lalu tambah provider satu per satu.

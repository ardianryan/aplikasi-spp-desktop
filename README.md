# Partisipasi Sekolah Desktop Client 🏫💳
**Aplikasi Kasir POS & Tabungan Sekolah Offline-First Berbasis Desktop**

[![Release](https://img.shields.io/badge/release-v0.1.0-blue.svg)](https://github.com/ardianryan/aplikasi-spp-desktop/releases/tag/v0.1.0)
[![Security](https://img.shields.io/badge/security-SQLCipher%20AES--256-emerald.svg)](SECURITY.md)
[![Compliance](https://img.shields.io/badge/compliance-UU%20PDP%20%7C%20UU%20ITE-002b59.svg)](SECURITY.md)
[![Stack](https://img.shields.io/badge/tech--stack-Tauri%20v2%20%7C%20Rust%20%7C%20React%2019-indigo.svg)](#-teknologi--arsitektur)

---

## 📌 Ringkasan Proyek

**Partisipasi Sekolah Desktop Client** adalah aplikasi kasir pembayaran dan manajemen keuangan sekolah berbasis desktop yang dirancang khusus untuk bekerja dalam kondisi jaringan tidak stabil (*offline-first*). Aplikasi ini melakukan enkripsi lokal penuh terhadap data keuangan dan data pribadi siswa, serta secara otomatis melakukan sinkronisasi dua arah (*bi-directional sync*) dengan server web pusat saat terhubung online.

---

## ⚡ Fitur Utama

- 🔒 **Basis Data Terenkripsi Penuh (SQLCipher AES-256):** Basis data SQLite lokal dienkripsi penuh menggunakan AES-256 dengan perlindungan *auto-recovery* otomatis untuk berkas tak terenkripsi.
- 🚀 **Penyiapan Tanpa Konfigurasi Manual (*Endpoint-First Zero-Config*):** Pengguna cukup memasukkan alamat URL situs web sekolah. Kunci otorisasi terminal kasir disediakan dan disinkronkan secara otomatis oleh peladen pusat tanpa perlu menyalin *API Key*.
- 🔑 **Autentikasi Cepat: Kode OTP 6-Digit & SSO Peramban (`psk://`):** Kasir dapat masuk instan menggunakan kode OTP 6-digit sekali pakai dari peramban atau melalui protokol *deep-link* aman.
- 📡 **Mesin Sinkronisasi Dua Arah (*Offline-First Sync Engine*):** Mengunduh data master siswa/tagihan dan mengunggah transaksi kasir secara otomatis di latar belakang, dilengkapi deteksi pulihnya jaringan (*auto-sync on online*).
- ⏳ **Pemantauan Antrean Transaksi Kasir (*Pending Sync Counter*):** Bilah atas kasir menampilkan lencana dinamis transaksi yang tersimpan secara lokal dan siap dikirim saat terhubung ke internet.
- 🖨️ **Cetak Kuitansi Offline dengan Kop Sekolah:** Aset logo sekolah dan stempel diunduh dan disimpan sebagai *Base64 Data URL* lokal agar kuitansi fisik dapat langsung dicetak saat internet terputus.
- 💰 **POS Kasir Cepat & Modul Tabungan Siswa:** Antarmuka pembayaran SPP, uang gedung, partisipasi, serta modul simpanan/penarikan tabungan siswa dengan kalkulasi otomatis real-time.
- 🧙 **Wizard Penyiapan 3 Langkah (*Onboarding Wizard*):** Panduan instalasi awal interaktif dengan uji sambungan peladen instan dan inisialisasi enkripsi basis data.

---

## ⚙️ Teknologi & Arsitektur

```
┌─────────────────────────────────────────────────────────────┐
│               React 19 + TypeScript + Vite                  │
│       (UI Layer: Vanilla CSS Design System, Inter Font)     │
└──────────────────────────────┬──────────────────────────────┘
                               │ IPC (Tauri Invoke Commands)
┌──────────────────────────────┴──────────────────────────────┐
│                    Tauri v2 Core Engine                     │
│               (Rust Async Engine & HTTP Client)             │
├──────────────────────────────┬──────────────────────────────┤
│    SQLite + SQLCipher AES    │   Reqwest TLS 1.3 Sync Client │
│   (Local Encrypted Database) │  (HTTPS API Pull / Push Engine)│
└──────────────────────────────┴──────────────────────────────┘
```

| Layer | Teknologi Utama |
|---|---|
| **Core Engine** | Rust 1.80+, Tauri v2 Framework |
| **Local Database** | SQLite 3 dengan `bundled-sqlcipher` (AES-256-CBC) via `rusqlite` |
| **Frontend Framework** | React 19, TypeScript, Vite v7 |
| **Design System** | Custom Vanilla CSS (Partisipasi Light Theme: Navy `#002b59`, Emerald `#008f5d`) |
| **Network & Sync** | `reqwest` (Async HTTP/TLS 1.3), `urlencoding`, `serde_json` |

---

## 🚀 Panduan Instalasi & Pengembangan

### Prasyarat Sistem
- **Node.js:** `>= 18.0.0`
- **Rust Toolchain:** `>= 1.75.0` (`rustc` & `cargo`)
- **System Dependencies:**
  - **macOS:** Xcode Command Line Tools (`xcode-select --install`)
  - **Linux (Ubuntu/Debian):** `libwebkit2gtk-4.1-dev build-essential curl wget file libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev`
  - **Windows:** C++ Build Tools & Windows SDK

### Langkah-Langkah Running Dev
1. **Clone repositori:**
   ```bash
   git clone https://github.com/nextlevelbuilder/partisipasi-desktop.git
   cd partisipasi-desktop
   ```

2. **Install dependency Node.js:**
   ```bash
   npm install
   ```

3. **Jalankan dalam mode Dev (Tauri + Vite):**
   ```bash
   npm run tauri dev
   ```

4. **Build untuk Produksi:**
   ```bash
   npm run tauri build
   ```

---

## 🧪 Pengujian & Penjaminan Mutu (QA)

Aplikasi ini dilengkapi dengan pengujian otomatis pada seluruh lapisan:

### Rust Unit & Integration Tests
```bash
cd src-tauri
cargo test --lib
```
*Menguji pengkodean Base64, parser DDL MySQL-ke-SQLite, eksekusi migrasi database 28 file in-memory, serta fungsi simpan/baca konfigurasi.*

### Backend Server Compatibility Tests (Pest PHP)
```bash
cd ../partisipasi-sekolah
./vendor/bin/pest
```
*Menguji API endpoint `/api/v1/sync/pull` dan `/api/v1/sync/push`.*

### Frontend Type-Checking & Build Validation
```bash
npm run build
```

---

## 🛡️ Kepatuhan Hukum & Keamanan

Implementasi keamanan teknis dan kepatuhan perundang-undangan didokumentasikan lengkap pada file **[SECURITY.md](SECURITY.md)**:
- **UU RI No. 27 Tahun 2022 tentang Pelindungan Data Pribadi (UU PDP)** — Pasal 35 & 39
- **UU RI No. 11 Tahun 2008 jo. UU No. 19 Tahun 2016 (UU ITE)** — Pasal 15 & 16
- **Standar Internasional:** ISO/IEC 27001:2022 (A.8.24 & A.8.5), OWASP DASVS V1 & V2, NIST SP 800-111

---

## 📜 Lisensi & Hak Cipta

Hak Cipta © 2026 **Partisipasi Sekolah Team**. Dipublikasikan sebagai perangkat lunak internal sekolah.

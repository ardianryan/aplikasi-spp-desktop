# Partisipasi Sekolah Desktop Client
**Aplikasi Kasir POS & Pembayaran Sumbangan Sekolah (Offline-First)**

[![Release](https://img.shields.io/badge/release-v0.2.0-blue.svg)](https://github.com/ardianryan/aplikasi-spp-desktop/releases/tag/v0.2.0)
[![Security](https://img.shields.io/badge/security-SQLCipher%20AES--256-emerald.svg)](SECURITY.md)
[![Compliance](https://img.shields.io/badge/compliance-UU%20PDP%20%7C%20UU%20ITE-002b59.svg)](SECURITY.md)
[![Stack](https://img.shields.io/badge/tech--stack-Slint%20UI%20%7C%20Rust%20%7C%20SQLCipher-indigo.svg)](#teknologi--arsitektur)

---

## Ringkasan Proyek

**Partisipasi Sekolah Desktop Client** adalah aplikasi kasir pembayaran dan pengelolaan keuangan sekolah berbasis desktop yang dikembangkan oleh **PMTI SMAN 1 GEDEG** untuk SMA Negeri 1 Gedeg. Aplikasi ini dirancang khusus untuk operasional kasir harian yang andal dalam berbagai kondisi jaringan (*offline-first*).

Aplikasi bekerja secara langsung menggunakan basis data terenkripsi di komputer lokal kasir tanpa bergantung pada peramban web (*browser*). Ketika komputer terhubung kembali dengan internet atau jaringan lokal sekolah, seluruh data pembayaran otomatis disinkronkan dua arah (*bi-directional sync*) dengan server web pusat (`https://partisipasi.sekolah.sch.id`).

---

## Fitur Utama

- **Antarmuka Slint UI Terintegrasi:** Berjalan langsung dengan konsumsi memori (*RAM*) yang sangat hemat, rendering responsif, serta waktu buka aplikasi yang instan.
- **Basis Data Terenkripsi Penuh (SQLCipher AES-256):** Seluruh data transaksi, identitas siswa, dan buku tabungan dienkripsi menggunakan AES-256 pada level basis data lokal. Dilengkapi mekanisme pencadangan otomatis berstempel waktu (`.corrupt_backup_{timestamp}.db`) untuk menjaga integritas data jika terjadi anomali berkas.
- **Penyiapan Tanpa Konfigurasi Manual (*Zero-Config Endpoint*):** Kasir cukup memasukkan alamat server sekolah. Kunci otorisasi terminal disediakan dan disinkronkan secara otomatis oleh server tanpa perlu menyalin *API Key* secara manual.
- **Autentikasi Cepat via Kode Otorisasi 6-Digit (OTP) & SSO:** Kasir dapat masuk secara instan menggunakan kode OTP 6-digit dari peramban atau melalui tautan otorisasi (*deep link*) `partisipasi-sekolah://`.
- **Dukungan Sinkronisasi Lengkap (Bertahap & Penuh):** Dilengkapi sinkronisasi berkala di latar belakang serta tombol **Sinkronisasi Penuh (Semua Data)** yang mampu merekonsiliasi seluruh data sekolah dalam waktu singkat (~11 detik). Layar kasir otomatis memuat ulang tagihan siswa aktif saat sinkronisasi selesai.
- **Pencetakan Kuitansi Standar (Printer Kantor A4):** Mencetak bukti pembayaran langsung melalui dialog cetak sistem operasi ke printer kasir/kantor (Inkjet, LaserJet, dsb.) dengan format HTML A4 1:1 identik dengan cetakan web resmi lengkap dengan kop, logo, dan stempel sekolah.
- **Kamus Istilah & Nomenklatur Dinamis:** Mengikuti konfigurasi istilah dari server web (misalnya istilah pembayaran dapat disesuaikan menjadi sumbangan atau partisipasi).
- **Siklus Kalender Akademik Sekolah (Juli – Juni):** Penataan urutan tagihan bulanan berstandar kalender tahun pelajaran sekolah yang dimulai dari bulan Juli hingga Juni.
- **Mode Berjalan di Latar Belakang (*System Tray*):** Menekan tombol tutup `[X]` otomatis menyembunyikan jendela ke baki sistem (*system tray*) agar proses sinkronisasi otomatis tetap beroperasi.
- **Wizard Penyiapan 3 Langkah:** Panduan penyiapan awal interaktif dengan uji sambungan server instan.

---

## Teknologi & Arsitektur

```
┌─────────────────────────────────────────────────────────────┐
│                          Slint UI                           │
│        (Antarmuka Pengguna: Kasir POS, Riwayat, Laporan)    │
└──────────────────────────────┬──────────────────────────────┘
                               │ Event Loop & Callback
┌──────────────────────────────┴──────────────────────────────┐
│                  Rust Application Engine                    │
│             (Runtime Asinkron Tokio & Single Instance)      │
├──────────────────────────────┬──────────────────────────────┤
│    SQLite + SQLCipher AES    │    Reqwest TLS 1.3 Client    │
│  (Basis Data Lokal Terenkripsi)│ (Sinkronisasi Dua Arah HTTPS)│
├──────────────────────────────┴──────────────────────────────┤
│             CUPS & Windows Print Adapter Spooler            │
│         (Pencetakan Kuitansi Fisik Langsung Tanpa Web)      │
└─────────────────────────────────────────────────────────────┘
```

| Komponen | Spesifikasi Teknologi |
| :--- | :--- |
| **Pengembang** | **PMTI SMAN 1 GEDEG** (SMA Negeri 1 Gedeg) |
| **Kerangka Antarmuka** | Slint UI 1.9 (arsitektur deklaratif berbasis Rust) |
| **Mesin Inti** | Rust 2021 Edition, Tokio Runtime, Tauri Core Services |
| **Basis Data Lokal** | SQLite 3 dengan enkripsi SQLCipher AES-256 (`bundled-sqlcipher` via `rusqlite`) |
| **Adapter Pencetakan** | CUPS (`lpr`) untuk macOS & Linux, spooler cetak untuk Windows |
| **Jaringan & Sinkronisasi** | `reqwest` (HTTPS / TLS 1.3), `serde_json`, `urlencoding` |
| **IPC & Instance Guard** | Unix Domain Socket (mencegah instance aplikasi ganda) |

---

## Panduan Instalasi & Pengembangan

### Prasyarat Sistem
- **Rust Toolchain:** `>= 1.75.0` (`rustc` dan `cargo`)
- **Dependensi Sistem:**
  - **macOS:** Xcode Command Line Tools (`xcode-select --install`)
  - **Linux (Ubuntu/Debian):** `build-essential libssl-dev libfontconfig1-dev libcups2-dev`
  - **Windows:** Visual Studio C++ Build Tools & Windows SDK

### Langkah-Langkah Menjalankan Proyek

1. **Klon Repositori:**
   ```bash
   git clone https://github.com/ardianryan/aplikasi-spp-desktop.git
   cd aplikasi-spp-desktop
   ```

2. **Jalankan dalam Mode Pengembangan:**
   ```bash
   cd src-tauri
   cargo run --bin partisipasi
   ```

3. **Kompilasi Biner Rilis (*Production Build*):**
   ```bash
   cd src-tauri
   cargo build --release --bin partisipasi
   ```
   Hasil biner siap pakai akan berada di `src-tauri/target/release/partisipasi`.

---

## Pengujian & Penjaminan Mutu

### Pengujian Unit & Integrasi Rust
```bash
cd src-tauri
cargo test --lib
```

### Pengujian Kesesuaian Peladen Web (Pest PHP)
```bash
cd ../partisipasi-sekolah
./vendor/bin/pest
```

---

## Kepatuhan Hukum & Keamanan

Implementasi teknis keamanan informasi dan perlindungan data pribadi siswa dirancang sesuai regulasi yang berlaku:
- **UU RI No. 27 Tahun 2022 tentang Pelindungan Data Pribadi (UU PDP):** Enkripsi lokal SQLCipher AES-256 menjamin data identitas dan keuangan siswa tetap terlindungi saat perangkat kasir mati atau tercuri secara fisik.
- **UU RI No. 11 Tahun 2008 jo. UU No. 19 Tahun 2016 (UU ITE):** Menjamin integritas dan keaslian bukti pembayaran elektronik.
- **Standar Keamanan:** ISO/IEC 27001:2022, OWASP DASVS, dan NIST SP 800-111. Informasi selengkapnya dapat dilihat pada **[SECURITY.md](SECURITY.md)**.

---

## Hak Cipta & Lisensi

Hak Cipta © 2026 **PMTI SMAN 1 GEDEG** (SMA Negeri 1 Gedeg).  
Dikelola dan dipublikasikan sebagai perangkat lunak operasional internal sekolah.

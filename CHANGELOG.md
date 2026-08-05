# Catatan Perubahan (CHANGELOG.md)

Semua perubahan penting pada proyek **Partisipasi Sekolah Desktop Client** akan didokumentasikan dalam file ini.

Format dokumen ini mengacu pada [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) dan proyek ini mematuhi [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] - 2026-08-05

### 🚀 Dibuat (Added)
- **Mesin Enkripsi SQLCipher AES-256-CBC:** Mengganti standar driver SQLite polos dengan `bundled-sqlcipher` pada layer Rust (`rusqlite`) untuk melindungi seluruh data transaksi dan identitas siswa saat tersimpan di perangkat lokal.
- **Key Derivation System:** Mekanisme pembentukan kunci enkripsi otomatis berdasarkan *API Key Terminal* yang terdaftar di server admin web.
- **Onboarding Wizard 3 Langkah:** Wizard instalasi pertama kali interaktif dengan visual stepper, pengujian koneksi API server real-time, dan inisialisasi enkripsi basis data.
- **Offline Receipt Asset Downloader (`download_school_assets`):** Fitur pengunduhan logo sekolah dan stempel dari server yang dikonversi langsung menjadi *Base64 Data URL* lokal agar pencetakan kwitansi fisik berfungsi tanpa koneksi internet.
- **Desain UI/UX Berstandar Brand Application:** Pembaruan penuh gaya antarmuka mengikuti token desain resmi Partisipasi Sekolah (Deep Navy `#002b59`, Emerald `#008f5d`, Light Mode `#faf8ff`, dan icon vector SVG clean).
- **Deep Link Browser SSO Protocol (`psk://`):** Integrasi alur masuk pengguna kasir melalui browser bawaan sistem secara aman.
- **Automated Test Suite (Pest PHP & Cargo Test):** Penambahan 5 unit test Rust in-memory database dan 11 unit & feature test Pest pada backend server.
- **Dokumentasi Keamanan & Kepatuhan Hukum (`SECURITY.md`):** Pembuatan pedoman keamanan teknis serta pemenuhan UU RI No. 27/2022 (UU PDP), UU ITE No. 19/2016, ISO 27001, OWASP DASVS, dan NIST SP 800-111.

### 🔄 Diubah (Changed)
- **Migrasi Database Auto-Clean:** Fungsi `clean_mysql_for_sqlite` diperbarui untuk mentranslasikan tipe data DDL MySQL (`ENUM`, `TINYINT`, komentar kolom, dan fungsi `UUID()`) secara otomatis ke sintaksis SQLite yang valid.
- **Gating Instalasi Pertama (`is_fresh_install`):** Mengganti modal aktivasi lama dengan gateway Onboarding Wizard sebelum pengguna diizinkan masuk ke aplikasi utama.

### 🐛 Diperbaiki (Fixed)
- **Tab Buku Tabungan Visibility Catch:** Menambahkan fallback `.catch(() => false)` pada pemanggilan command `is_savings_enabled` untuk mencegah tabungan muncul tidak disengaja saat terjadi kegagalan pembacaan konfigurasi awal.
- **Kemunculan Logo Kop Kwitansi saat Offline:** Mengatasi kendala gambar logo kop kwitansi hilang/broken saat cetak tanpa koneksi internet dengan penyimpanan Base64 lokal.

---

[0.1.0]: https://github.com/nextlevelbuilder/partisipasi-desktop/releases/tag/v0.1.0

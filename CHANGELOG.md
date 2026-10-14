# Catatan Perubahan (CHANGELOG.md)

Semua perubahan penting pada proyek **Partisipasi Sekolah Desktop Client** akan didokumentasikan dalam file ini.

Format dokumen ini mengacu pada [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) dan proyek ini mematuhi [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2026-09-30

### 🚀 Dibuat (Added)
- **Arsitektur Autentikasi Endpoint-First (Zero-Config API Key):** Pengguna tidak perlu lagi menyalin dan menempel *API Key* secara manual. Klien desktop cukup mengarahkan alamat web sekolah, lalu masuk via peramban (SSO) atau memasukkan kode OTP 6-digit. *API Key* sekolah langsung disediakan dan disimpan secara otomatis (*auto-provisioning*) ke basis data lokal.
- **Masuk Cepat Kasir via Kode OTP 6-Digit:** Implementasi perintah `login_with_pairing_code` yang menukarkan kode otorisasi 6-digit dari peramban ke server dengan proteksi *single-use token* dan pembatasan percobaan gagal.
- **Dynamic Per-Device Salt untuk PIN Kasir Offline:** Implementasi pengacakan unik per perangkat (`cashier_offline_pin_salt`) untuk hash SHA-256 PIN offline kasir, memberikan ketahanan maksimal terhadap serangan *rainbow table*.
- **Restriksi Hak Akses Berkas Seed Kunci (`chmod 0600`):** Berkas `.cipher_seed` pada sistem operasi macOS/Linux otomatis dikunci dengan izin hanya dapat dibaca dan ditulis oleh pemilik proses (*owner read-write only*).
- **Indikator & Pemantauan Antrean Sinkronisasi Kasir:** Penambahan perintah `get_pending_sync_count` dan lencana dinamis pada bilah atas antarmuka kasir (`✓ Data sinkron` vs `⏳ X transaksi tertunda`).
- **Sinkronisasi Otomatis saat Jaringan Pulih (*Network Online Listener*):** Penambahan pendengar kejadian `window.addEventListener('online')` yang secara otomatis memicu sinkronisasi latar belakang saat internet kembali tersambung.
- **Aksesibilitas Keyboard Modal (Standar R-32):** Seluruh jendela dialog/modal (kuitansi, pengaturan PIN kasir, ubah sambungan) kini dapat ditutup dengan tombol keyboard `Escape`.
- **Rincian Riwayat Cicilan Tagihan Bebas:** Menampilkan riwayat pembayaran bertahap (cicilan) pada pos tagihan bebas di tabel kasir desktop dengan tombol pengalih (*accordion toggle*) `Riwayat Cicilan (Nx)`, menampilkan urutan pembayaran, tanggal/waktu transaksi, dan nominal cicilan secara rinci.
- **Dukungan Ikon Baki Sistem (*System Tray*) & Mode Latar Belakang (Failover Daemon):** Menambahkan baki sistem (*tray icon*) ala Warp Tunnel / Tailscale dengan menu klik-kanan ("Buka Partisipasi Sekolah", "Sinkronkan Sekarang", dan "Keluar").
- **Tutup-ke-Baki (*Minimize to System Tray*):** Tombol tutup jendela `[X]` kini otomatis menyembunyikan jendela ke baki sistem tanpa mematikan proses, memastikan mesin sinkronisasi terus berjalan.
- **Autostart saat Komputer Dinyalakan:** Integrasi `@tauri-apps/plugin-autostart` yang secara otomatis mengaktifkan peluncuran otomatis aplikasi saat kasir berhasil masuk, lengkap dengan sakelar manual di tab Pengaturan Sambungan & Sinkronisasi.
- **Pembaruan Otomatis Hening di Latar Belakang (*Silent Auto-Updater ala Antigravity IDE*):** Integrasi `@tauri-apps/plugin-updater` dan `@tauri-apps/plugin-process` yang secara berkala memeriksa rilis baru di GitHub Releases, mengunduh serta memasang berkas instalasi di latar belakang tanpa mengganggu transaksi kasir, lalu memunculkan kartu mengambang (*floating banner*) elegan untuk melakukan *restart* dan membuka versi terbaru secara instan.
- **Tanda Tangan Keamanan Pembaruan (Minisign Cryptographic Verification):** Menambahkan verifikasi kunci publik Minisign pada `tauri.conf.json` dan workflow GitHub Actions untuk menjamin integritas paket instalasi pembaruan.

### 🔄 Diubah (Changed)
- **Pembersihan Antarmuka Pengguna (Anti-Slop UI Hygiene):** Menghapus seluruh kolom isian *API Key* / *X-API-Key* dari wizard penyiapan awal (Langkah 2), modal ubah sambungan, dan tab pengaturan kasir ("Sync & Koneksi").
- **Standarisasi Bahasa Indonesia Baku (EYD Edisi Kelima):** Penyesuaian seluruh peristilahan teknis pada antarmuka kasir dan kuitansi (mengganti *kwitansi* menjadi *kuitansi*, *browser* menjadi *peramban*, *server* menjadi *peladen*, serta menghapus pemakaian tanda pisah *em dash* yang tidak baku).

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

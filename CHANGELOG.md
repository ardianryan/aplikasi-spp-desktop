# Catatan Perubahan (CHANGELOG.md)

Semua perubahan penting pada proyek **Partisipasi Sekolah Desktop Client** didokumentasikan dalam berkas ini.

Format dokumen ini mengacu pada [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) dan proyek ini mematuhi [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.2.1] - 2026-11-05

### Dibuat (Added)
- **Desain Baru Ikon & Identitas Aplikasi Partisipasi Sekolah:** Merilis set ikon resmi berstandar modern berbasis vektor geometris (monogram P beraksen gotong-royong, pilar pendidikan, dan bintang capaian akademik), menggantikan ikon bawaan templat dan mendukung seluruh resolusi platform (macOS ICNS, Windows ICO, dan set PNG 16x16 hingga 512x512).
- **Alur Mandiri Pencetakan Kuitansi (*In-App*):**
  - Dialog pratinjau kuitansi di layar kasir otomatis menutup seketika saat tombol cetak ditekan (*auto-dismiss*), mengembalikan fokus kasir ke transaksi berikutnya tanpa perlu menekan tombol tutup terpisah.
  - Dialog cetak pada Windows dialihkan ke mode jendela mandiri (*App-Mode*) dengan parameter penutupan otomatis pasca-cetak (`window.onafterprint = window.close()`), mencegah terbukanya peramban luar dengan bilah tautan atau deretan tab.
- **Pembersihan Istilah Terlarang:** Menghapus seluruh residu istilah non-standar pada penamaan fungsi perantara (`run_slint_desktop`) demi kepatuhan penuh terhadap pedoman pengembangan proyek.

---

## [0.2.0] - 2026-11-04

### Dibuat (Added)
- **Pembaruan Kerangka Antarmuka Slint UI:** Memigrasikan antarmuka grafis ke Slint UI berbasis Rust, menghasilkan waktu luncur instan, performa rendering tinggi, serta konsumsi memori (*RAM*) yang sangat hemat dibandingkan versi peramban sebelumnya.
- **Paritas 1:1 Pembayaran Kasir POS:** Pemisahan sub-kelompok *Tagihan Bebas* dan *Tagihan Bulanan* per tahun pelajaran, penambahan kolom catatan transaksi kasir, dukungan riwayat angsuran cicilan pos bebas, badge lunas berformat tanggal bayar terakhir (`LUNAS: dd/mm/YYYY`), serta pembakuan format kode transaksi menjadi `TRX-YYYYMMDD-XXXXXX`.
- **Pencatatan Otomatis Jurnal Kas Masuk:** Setiap transaksi pembayaran kasir lokal otomatis mencatat pembukuan kas masuk (`type = 'pemasukan'`) ke tabel `journals` lokal yang otomatis terdorong ke peladen saat proses sinkronisasi.
- **Penyelarasan Format Kuitansi Digital & Fisik:** Menghormati konfigurasi `show_nominal_receipt` dari pengaturan sekolah (menampilkan status lunas bila nominal disembunyikan), penyelarasan format keterangan pos kuitansi, serta penataan ulang lebar daftar dropdown pencarian siswa agar tombol aksi tidak terpotong.
- **Adapter Pencetakan Kuitansi Fisik Langsung:** Integrasi adapter cetak langsung berbasis CUPS (`lpr`) di macOS/Linux dan spooler sistem di Windows. Kuitansi dicetak langsung ke printer kasir USB/LAN tanpa perlu membuka jendela PDF atau peramban web, dengan tata letak 1:1 identik dengan versi web.
- **Fitur Sinkronisasi Penuh (*Full Baseline Sync*):** Penambahan tombol *Sinkronisasi Penuh (Semua Data)* di menu Pengaturan yang mampu menarik dan merekonsiliasi seluruh data acuan dan riwayat transaksi sekolah (~56 MB) dalam tempo ~11 detik melalui transaksi SQLite teroptimasi.
- **Penyegaran Otomatis Layar Kasir POS:** Tampilan kasir otomatis memuat ulang rincian tagihan siswa aktif sesaat setelah sinkronisasi (baik manual maupun otomatis di latar belakang) selesai tanpa perlu pencarian ulang.
- **Urutan Kalender Akademik Sekolah:** Penataan urutan tagihan bulanan berstandar tahun ajaran sekolah dimulai dari bulan Juli hingga Juni.
- **Pencegahan Instans Ganda (*Single Instance Lock*):** Menggunakan Unix Domain Socket IPC lokal untuk memastikan hanya ada satu instans aplikasi yang aktif dan meneruskan fokus saat pengguna membuka aplikasi kembali.
- **Penyelamatan Basis Data Otomatis:** Mengganti penghapusan berkas rusak dengan pembuatan salinan cadangan otomatis berstempel waktu (`.corrupt_backup_{timestamp}.db`).

### Diubah (Changed)
- **Perbaikan Mesin Sinkronisasi Tagihan (`payment_assignments`):** Menghilangkan penguncian status lokal `pending_push` pada `payment_assignments` yang sebelumnya menghalangi pembaruan data tagihan siswa dari peladen web, otomatis memulihkan baris terisolir pada proses *pull*.
- **Penegakan Default Protokol HTTPS:** Seluruh komunikasi jaringan ke peladen pusat (`https://partisipasi.sekolah.sch.id`) kini menggunakan enkripsi TLS/HTTPS secara bawaan guna mencegah risiko penyadapan data keuangan pada jaringan lokal sekolah.
- **Restriksi Hak Akses Push Sync:** Membatasi endpoint `syncPush` di peladen pusat dengan *allowlist* khusus tabel transaksi kasir (`payments`, `payment_details`, `reliefs`, `journals`, `savings`, `savings_transactions`), menolak penimpaan data pengguna atau pengaturan sistem sekolah.
- **Perbaikan Keamanan Rate Limiting OTP:** Reset batas laju pada `exchangeDesktopPairingCode` kini hanya dieksekusi setelah kode otorisasi 6-digit terbukti valid, belum pernah dipakai, dan belum kedaluwarsa.
- **Penyensoran Kredensial Log:** Menyensor token otorisasi pada catatan log konsol deep-link untuk mencegah kebocoran informasi sensitif.
- **Pembersihan Anti-Slop (Bahasa & Komentar Kode):**
  - Menghapus klaim bombastis, pleonasma ("header atas"), dan emoji dekoratif pada tombol web.
  - Menghapus pembatas banner garis dekoratif dan penomoran alur mekanis pada basis kode sumber.
  - Menetapkan identitas resmi tim pengembang: **PMTI SMAN 1 GEDEG**.

---

## [0.1.1] - 2026-09-30

### Dibuat (Added)
- **Arsitektur Autentikasi Endpoint-First (Zero-Config API Key):** Pengguna tidak perlu lagi menyalin dan menempel *API Key* secara manual. Klien desktop cukup mengarahkan alamat web sekolah, lalu masuk via peramban (SSO) atau memasukkan kode OTP 6-digit. *API Key* sekolah langsung disediakan dan disimpan secara otomatis (*auto-provisioning*) ke basis data lokal.
- **Masuk Cepat Kasir via Kode OTP 6-Digit:** Implementasi perintah `login_with_pairing_code` yang menukarkan kode otorisasi 6-digit dari peramban ke server dengan proteksi *single-use token* dan pembatasan percobaan gagal.
- **Dynamic Per-Device Salt untuk PIN Kasir Offline:** Implementasi pengacakan unik per perangkat (`cashier_offline_pin_salt`) untuk hash SHA-256 PIN offline kasir, memberikan ketahanan maksimal terhadap serangan *rainbow table*.
- **Restriksi Hak Akses Berkas Seed Kunci (`chmod 0600`):** Berkas `.cipher_seed` pada sistem operasi macOS/Linux otomatis dikunci dengan izin hanya dapat dibaca dan ditulis oleh pemilik proses (*owner read-write only*).
- **Indikator & Pemantauan Antrean Sinkronisasi Kasir:** Penambahan perintah `get_pending_sync_count` dan lencana dinamis pada bilah atas antarmuka kasir.
- **Sinkronisasi Otomatis saat Jaringan Pulih (*Network Online Listener*):** Penambahan pendengar kejadian jaringan yang secara otomatis memicu sinkronisasi latar belakang saat internet kembali tersambung.
- **Aksesibilitas Keyboard Modal:** Seluruh jendela dialog/modal dapat ditutup dengan tombol keyboard `Escape`.
- **Rincian Riwayat Cicilan Tagihan Bebas:** Menampilkan riwayat pembayaran bertahap (cicilan) pada pos tagihan bebas di tabel kasir desktop.
- **Dukungan Ikon Baki Sistem (*System Tray*) & Mode Latar Belakang:** Menambahkan baki sistem (*tray icon*) dengan menu klik-kanan ("Buka Partisipasi Sekolah", "Sinkronkan Sekarang", dan "Keluar").
- **Tutup-ke-Baki (*Minimize to System Tray*):** Tombol tutup jendela `[X]` otomatis menyembunyikan jendela ke baki sistem tanpa mematikan proses, memastikan mesin sinkronisasi terus berjalan.
- **Autostart saat Komputer Dinyalakan:** Integrasi peluncuran otomatis aplikasi saat kasir berhasil masuk.
- **Pembaruan Otomatis Hening di Latar Belakang (*Silent Auto-Updater*):** Memeriksa rilis baru di GitHub Releases, mengunduh serta memasang berkas instalasi di latar belakang tanpa mengganggu transaksi kasir.
- **Tanda Tangan Keamanan Pembaruan (Minisign Cryptographic Verification):** Menambahkan verifikasi kunci publik Minisign pada `tauri.conf.json`.

### Diubah (Changed)
- **Pembersihan Antarmuka Pengguna (Anti-Slop UI Hygiene):** Menghapus kolom isian *API Key* manual dari antarmuka kasir.
- **Standarisasi Bahasa Indonesia Baku (EYD V):** Penyesuaian peristilahan teknis pada antarmuka kasir dan kuitansi sesuai kaidah bahasa baku formal.

---

## [0.1.0] - 2026-08-05

### Dibuat (Added)
- **Mesin Enkripsi SQLCipher AES-256-CBC:** Mengganti standar driver SQLite polos dengan `bundled-sqlcipher` pada layer Rust (`rusqlite`) untuk melindungi seluruh data transaksi dan identitas siswa saat tersimpan di perangkat lokal.
- **Key Derivation System:** Mekanisme pembentukan kunci enkripsi otomatis berdasarkan *API Key Terminal* yang terdaftar di server admin web.
- **Onboarding Wizard 3 Langkah:** Wizard instalasi pertama kali interaktif dengan visual stepper, pengujian koneksi API server real-time, dan inisialisasi enkripsi basis data.
- **Offline Receipt Asset Downloader (`download_school_assets`):** Fitur pengunduhan logo sekolah dan stempel dari server yang dikonversi langsung menjadi *Base64 Data URL* lokal agar pencetakan kuitansi fisik berfungsi tanpa koneksi internet.
- **Desain UI/UX Berstandar Brand Application:** Pembaruan penuh gaya antarmuka mengikuti token desain resmi Partisipasi Sekolah (Deep Navy `#002b59`, Emerald `#008f5d`, Light Mode `#faf8ff`, dan icon vector SVG clean).
- **Deep Link Browser SSO Protocol:** Integrasi alur masuk pengguna kasir melalui peramban bawaan sistem secara aman.
- **Automated Test Suite (Pest PHP & Cargo Test):** Penambahan unit test Rust in-memory database dan unit & feature test Pest pada backend server.
- **Dokumentasi Keamanan & Kepatuhan Hukum (`SECURITY.md`):** Pembuatan pedoman keamanan teknis serta pemenuhan UU RI No. 27/2022 (UU PDP), UU ITE No. 19/2016, ISO 27001, OWASP DASVS, dan NIST SP 800-111.

---

[0.2.0]: https://github.com/ardianryan/aplikasi-spp-desktop/releases/tag/v0.2.0
[0.1.1]: https://github.com/ardianryan/aplikasi-spp-desktop/releases/tag/v0.1.1
[0.1.0]: https://github.com/ardianryan/aplikasi-spp-desktop/releases/tag/v0.1.0

# Kebijakan & Spesifikasi Keamanan (SECURITY.md)
**Aplikasi Desktop Partisipasi Sekolah**  
*Sistem Kasir POS & Tabungan Sekolah Client-Side*

---

## 🔒 1. Arsitektur Keamanan Teknis

Aplikasi Desktop **Partisipasi Sekolah** menerapkan strategi *Defense-in-Depth* untuk menjamin kerahasiaan (*Confidentiality*), integritas (*Integrity*), dan ketersediaan (*Availability*) data pembayaran serta data pribadi siswa pada terminal kasir desktop.

### A. Enkripsi Data Saat Tersimpan (Data-at-Rest Encryption)
- **Teknologi Enkripsi:** **SQLCipher AES-256-CBC** dengan *PBKDF2 key derivation*.
- **Cakupan Enkripsi:** Seluruh isi basis data SQLite lokal (`partisipasi.db`), termasuk data identitas siswa, tunggakan, transaksi kasir POS, dan buku tabungan.
- **Kunci Enkripsi:** Diderivasi dari *API Key Terminal* unik yang dipadukan dengan *device-unique seed* berentropi tinggi.
- **Proteksi Akses:** File `.db` yang tersimpan tidak dapat dibuka atau di-dump menggunakan CLI SQLite biasa maupun perkakas DB Browser (mengembalikan error `file is not a database`).

### B. Keamanan Komunikasi Jaringan (Data-in-Transit Encryption)
- **Protokol:** Enkripsi wajib **TLS 1.2 / 1.3 (HTTPS)** untuk seluruh komunikasi antara aplikasi desktop dan server API pusat.
- **Otentikasi API:** Menggunakan header kustom `X-API-Key` berbasis hash token untuk setiap request *Sync Pull* dan *Sync Push*.
- **SSO Browser Binding:** Proses login pengguna kasir dan administrator dilakukan melalui browser bawaan sistem (System Browser SSO) menggunakan mekanisme **Deep Link Custom Scheme Protocol (`psk://`)**. Hal ini mencegah serangan *embedded webview credential harvesting* dan *man-in-the-middle (MITM)*.

### C. Proteksi Injeksi & Integritas Data
- **Parameterized Queries:** Seluruh operasi database di layer Rust (`rusqlite`) menggunakan *prepared statements* terparameter untuk mencegah serangan *SQL Injection*.
- **Integritas Relasional:** Penerapan PRAGMA `foreign_keys = ON` dan *UUID v4 GUID primary keys* untuk mencegah konflik ID atau manipulasi data relasional lokal.
- **Jejak Audit Sinkronisasi:** Setiap record transaksi lokal memiliki atribut `sync_status` (`pending_push` / `synced`) dan timestamp yang menjamin keterlacakan audit (*audit trail*).

---

## 🇮🇩 2. Kepatuhan Terhadap Perundang-Undangan Republik Indonesia

Aplikasi ini dirancang untuk mematuhi regulasi perundang-undangan digital yang berlaku di Indonesia:

### A. UU No. 27 Tahun 2022 tentang Pelindungan Data Pribadi (UU PDP)
- **Pasal 35 (Keamanan Data Pribadi):** Pengendali data wajib melindungi data pribadi dari pemrosesan tanpa hak. Penggunaan **SQLCipher AES-256** memastikan data pribadi siswa (NISN, Nama, Kelas, Wali) dan riwayat transaksi finansial sekolah tidak dapat diakses pihak ketiga jika perangkat laptop/PC kasir mengalami pencurian atau kehilangan fisik.
- **Pasal 39 (Pencegahan Kegagalan Pelindungan Data):** Mencegah kebocoran data (*data breach*) pada terminal komputer kasir lokal dengan enkripsi otomatis saat aplikasi ditutup maupun berjalan.
- **Pasal 16 & 28 (Prinsip Pemrosesan Data):** Data siswa yang diunduh lokal hanya terbatas pada data yang dibutuhkan untuk operasional kasir dan pembayaran sekolah.

### B. UU No. 11 Tahun 2008 jo. UU No. 19 Tahun 2016 tentang ITE
- **Pasal 15 (Penyelenggaraan Sistem Elektronik secara Aman & Andal):** Sistem kasir desktop menjamin keandalan transaksi elektronik dengan pencatatan status sinkronisasi yang tidak manipulatif.
- **Pasal 16 (Keutuhan Informasi Elektronik):** Kwitansi dan bukti pembayaran yang dicetak atau tersimpan dijamin keasliannya melalui checksum pencatatan di basis data terenkripsi.

---

## 🌐 3. Kepatuhan Terhadap Standar Keamanan Internasional

| Standar Keamanan | Kontrol / Kategori | Implementasi pada Aplikasi Desktop |
|---|---|---|
| **ISO/IEC 27001:2022** | **A.8.24 (Use of Cryptography)** | Penggunaan enkripsi kuat AES-256 untuk *storage* dan TLS 1.2+ untuk *transit*. |
| **ISO/IEC 27001:2022** | **A.8.5 (Secure Authentication)** | Integrasi SSO Browser pihak pertama dengan proteksi Deep Link Protocol. |
| **OWASP DASVS** | **V1: Architecture & Design** | Pemisahan tegas antara kredensial pengguna dan kunci enkripsi basis data. |
| **OWASP DASVS** | **V2: Data Storage Security** | Bebas dari penyimpanan teks biasa (*plain-text*) untuk data sensitif di lokal disk. |
| **NIST SP 800-111** | **End-User Device Storage** | Pemenuhan rekomendasi NIST untuk enkripsi penuh file basis data pada perangkat *end-user*. |

---

## 🛠️ 4. Panduan Penanganan Insiden & Pelaporan Kerentanan

Pihak sekolah atau pengembang yang menemukan potensi celah keamanan pada aplikasi desktop ini dapat melaporkannya melalui alur penanganan responsif (*Coordinated Vulnerability Disclosure*):

1. **Email Pelaporan:** `security@partisipasi.sch.id`
2. **Format Pelaporan:** Sertakan deskripsi kerentanan, langkah-langkah reproduksi (*Proof of Concept*), serta dampak potensial.
3. **Waktu Response:** Tim Keamanan Cyber Partisipasi Sekolah akan memberikan konfirmasi awal dalam 1x24 jam dan pembaruan patch keamanan dalam waktu maksimal 7 hari kerja.

# 📡 OpenAPI 3.0 Specification — RESTful Sync & Student Payment API

Dokumentasi API untuk integrasi aplikasi pendukung dan sinkronisasi data offline-ke-online pada aplikasi Pembayaran Sekolah (Partisipasi Sekolah).

> **Versi API:** `v1`
> **Format Data:** `JSON`
> **Header Autentikasi:** `X-API-Key`
> **Protokol CORS:** Didukung penuh (`GET`, `POST`, `OPTIONS`)
> **Terakhir Diperbarui:** 2026-08-05

---

## 🔐 Autentikasi & Otorisasi

Seluruh request API wajib menyertakan header berikut:
```http
X-API-Key: psk_live_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
```

Alternatif via query string (tidak dianjurkan untuk produksi):
```
GET /api/v1/sync/pull?api_key=psk_live_xxx
```

> API Key dapat dilihat dan diperbarui di **Pengaturan → Integrasi & API** pada panel administrator.

---

## 📌 Daftar Endpoint

### 1. `GET /api/v1/payments/student`

Membaca ringkasan tagihan, rincian item, dan riwayat pembayaran **siswa** atau **staf (guru/tendik)** berdasarkan kriteria pencarian unik.

#### Query Parameters

| Parameter          | Tipe     | Wajib | Keterangan |
| :----------------- | :------- | :---- | :--------- |
| `uuid`             | `string` | ❌ *  | UUID siswa |
| `nisn`             | `string` | ❌ *  | NISN siswa |
| `nip`              | `string` | ❌ *  | NIP guru/tendik — mengembalikan profil staf, bukan tagihan |
| `academic_year_id` | `string` | ❌    | Batasi pada T.A. tertentu (default: T.A. aktif) |

> **\*** Tepat **satu** dari `uuid`, `nisn`, atau `nip` wajib diisi. Mengirimkan lebih dari satu atau tanpa parameter akan menghasilkan `422 Unprocessable Entity`.

---

#### Response — Data Siswa (200 OK)

```json
{
  "success": true,
  "meta": {
    "source": "partisipasi-sekolah",
    "queried_by": "nisn",
    "academic_year": {
      "id": "8d3h-7ns8-29sj-10dm",
      "name": "2024/2025",
      "is_active": 1
    },
    "generated_at": "2026-08-05T13:00:00+07:00"
  },
  "data": {
    "student": {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "nis": "2024.10.001",
      "nisn": "0051234567",
      "nama": "Ananda Pratama",
      "kelas": "X-TKJ-1",
      "academic_year_id": "8d3h-7ns8-29sj-10dm"
    },
    "summary": {
      "total_tagihan": 1200000.0,
      "total_dibayar": 600000.0,
      "total_keringanan": 200000.0,
      "total_tunggakan": 400000.0
    },
    "assignments": [
      {
        "id": "90bba468-4355-4c3d-bf84-d4851119ad10",
        "payment_type_id": "pt-sumbangan-1",
        "payment_type_name": "Sumbangan Masyarakat",
        "payment_type_type": "bulanan",
        "month": 7,
        "month_name": "Juli",
        "amount": 200000.0,
        "paid_amount": 200000.0,
        "relief_amount": 0.0,
        "remaining_amount": 0.0,
        "status": "lunas"
      }
    ],
    "payments": [
      {
        "id": "payment-uuid-123",
        "transaction_code": "TX-20260805-001",
        "total_amount": 200000.0,
        "payment_method_name": "Tunai (Kasir)",
        "notes": "Pembayaran bulan Juli",
        "created_at": "2026-08-05 09:30:00",
        "items": [
          {
            "id": "detail-uuid-1",
            "payment_type_name": "Sumbangan Masyarakat",
            "payment_type_type": "bulanan",
            "month": 7,
            "month_name": "Juli",
            "amount": 200000.0
          }
        ],
        "receipt_url": "https://yourschool.sch.id/payments/receipt/payment-uuid-123"
      }
    ]
  }
}
```

#### Response — Data Staf via `nip` (200 OK)

```json
{
  "success": true,
  "meta": {
    "source": "partisipasi-sekolah",
    "queried_by": "nip",
    "member_type": "staff",
    "generated_at": "2026-08-05T13:00:00+07:00"
  },
  "data": {
    "staff": {
      "id": "user-uuid-guru-1",
      "nip": "198505152010011001",
      "nama": "Budi Santoso, S.Pd",
      "email": "budi@sekolah.sch.id",
      "sso_role": "guru",
      "role": "wali_kelas",
      "is_active": 1
    }
  }
}
```

> **Catatan:** Endpoint `nip` hanya mengembalikan profil staf — tidak ada data tagihan/pembayaran. User yang ditemukan harus memiliki `sso_role` = `guru` atau `tendik`, selain itu akan mendapat `403 Forbidden`.

#### Response Error

| HTTP Status | Kondisi |
| :---------- | :------ |
| `401` | API Key tidak valid atau tidak dikirimkan |
| `403` | NIP ditemukan tetapi user bukan guru/tendik |
| `404` | Siswa/staf/tahun pelajaran tidak ditemukan |
| `422` | Parameter tidak valid (0 atau lebih dari 1 parameter terisi) |

---

### 2. `GET /api/v1/sync/pull`

Mengunduh data dari server online ke desktop client. Mendukung **full pull** (tanpa `since`) dan **incremental pull** (dengan `since`) untuk efisiensi transfer.

> ⚠️ Full pull pertama kali (`since = 1970-01-01 00:00:00`) bisa menghabiskan memori jika database besar. Server sudah dikonfigurasi dengan `memory_limit = 1024M` untuk mengatasinya.

#### Query Parameters

| Parameter | Tipe     | Wajib | Keterangan |
| :-------- | :------- | :---- | :--------- |
| `since`   | `string` | ❌    | Format `YYYY-MM-DD HH:MM:SS`. Jika diisi, hanya mengembalikan record yang `updated_at >= since` atau `created_at >= since`. Jika kosong, mengembalikan seluruh data (full pull). |

#### Contoh Response (200 OK)

```json
{
  "success": true,
  "timestamp": "2026-08-05 13:00:00",
  "data": {
    "academic_years": [
      {
        "id": "8d3h-7ns8-29sj-10dm",
        "name": "2024/2025",
        "is_active": 1,
        "created_at": "2024-06-01 00:00:00",
        "updated_at": "2026-03-30 14:00:00"
      }
    ],
    "classrooms": [ { "id": "c-1", "name": "X-TKJ-1", "..." : "..." } ],
    "users": [],
    "students": [],
    "payment_types": [],
    "payment_methods": [],
    "payment_assignments": [],
    "payments": [],
    "payment_details": [],
    "reliefs": [],
    "journals": [],
    "savings": [],
    "savings_transactions": [],
    "debts": [],
    "settings": [],
    "student_academic_histories": [],
    "online_payment_orders": [],
    "online_payment_items": []
  }
}
```

---

### 3. `POST /api/v1/sync/push`

Mengirim data yang dibuat/diperbarui di desktop client (offline) ke database server online. Endpoint ini melakukan operasi **Upsert** (insert jika belum ada, update jika sudah ada) secara transaksional.

#### Perilaku Server

| Fitur | Detail |
| :---- | :----- |
| **Urutan tabel** | Server memproses tabel sesuai urutan dependency FK (parent dahulu, child kemudian) |
| **FK Safety** | `SET FOREIGN_KEY_CHECKS = 0` diaktifkan selama proses push untuk mencegah FK violation akibat urutan data yang tidak sempurna |
| **Best-effort** | Record yang gagal di-upsert dicatat di `warnings[]` dan dilewati — proses tidak berhenti total |
| **Rekonsiliasi otomatis** | Setelah push selesai, server menghitung ulang `paid_amount`, `status` tagihan, dan `balance` tabungan berdasarkan data aktual |
| **Normalisasi ENUM** | Nilai enum yang berbeda antara desktop (SQLite) dan server (MySQL) dinormalisasi otomatis (lihat tabel di bawah) |

#### Normalisasi Nilai ENUM Otomatis

| Tabel | Kolom | Nilai Desktop | Nilai MySQL |
| :---- | :---- | :------------ | :---------- |
| `journals` | `type` | `debet` | `pemasukan` |
| `journals` | `type` | `kredit` / lainnya | `pengeluaran` |
| `savings_transactions` | `type` | `deposit` / `setor` | `setor` |
| `savings_transactions` | `type` | `withdraw` / lainnya | `tarik` |
| `payment_assignments` | `status` | `belum_lunas` | `belum_bayar` |
| `payment_assignments` | `status` | `potongan` | `lunas` |
| `payment_assignments` | `status` | `paid` / `free` / lainnya tidak dikenal | dipetakan ke ENUM valid terdekat |
| `debts` | `type` | `hutang` / `piutang` | diteruskan langsung |
| `reliefs` | `type` | `free` | `pembebasan` |
| `reliefs` | `status` | nilai tidak dikenal | `pending` |

#### Nilai ENUM yang Valid di MySQL

| Tabel | Kolom | Nilai Valid |
| :---- | :---- | :---------- |
| `payment_assignments` | `status` | `belum_bayar` \| `cicilan` \| `lunas` \| `bebas` |
| `journals` | `type` | `pemasukan` \| `pengeluaran` |
| `savings_transactions` | `type` | `setor` \| `tarik` |
| `debts` | `type` | `hutang` \| `piutang` |
| `debts` | `status` | `belum_lunas` \| `lunas` |
| `reliefs` | `type` | `pembebasan` \| `potongan` |
| `reliefs` | `status` | `pending` \| `approved` \| `rejected` \| `canceled` |

#### Rekonsiliasi Otomatis Setelah Push

Setelah semua record diupsert, server menjalankan 3 query rekonsiliasi:

1. **`payment_assignments.paid_amount`** — dihitung ulang dari `SUM(payment_details.amount)` aktual
2. **`payment_assignments.status`** — dievaluasi ulang:
   - `bebas` jika `relief_amount >= amount`
   - `lunas` jika `paid_amount + relief_amount >= amount`
   - `cicilan` jika `paid_amount > 0`
   - `belum_bayar` untuk sisanya
3. **`savings.balance`** — dihitung ulang dari mutasi `savings_transactions` aktual

#### Request Body (JSON)

Kirimkan objek JSON dengan key = nama tabel, value = array records:

```json
{
  "payments": [
    {
      "id": "payment-uuid-999",
      "academic_year_id": "8d3h-7ns8-29sj-10dm",
      "student_id": "550e8400-e29b-41d4-a716-446655440000",
      "transaction_code": "TX-LOCAL-999",
      "total_amount": 200000,
      "payment_method_name": "Tunai (Kasir)",
      "notes": "Pembayaran lokal disinkronkan",
      "created_at": "2026-08-05 11:15:00",
      "updated_at": "2026-08-05 11:15:00"
    }
  ],
  "payment_details": [
    {
      "id": "detail-uuid-999",
      "payment_id": "payment-uuid-999",
      "payment_assignment_id": "90bba468-4355-4c3d-bf84-d4851119ad10",
      "amount": 200000,
      "created_at": "2026-08-05 11:15:00",
      "updated_at": "2026-08-05 11:15:00"
    }
  ],
  "journals": [
    {
      "id": "journal-uuid-1",
      "type": "debet",
      "amount": 200000,
      "description": "Penerimaan SPP Juli",
      "created_at": "2026-08-05 11:15:00"
    }
  ],
  "savings_transactions": [
    {
      "id": "st-uuid-1",
      "savings_id": "savings-uuid-1",
      "type": "deposit",
      "amount": 50000,
      "description": "Setor tabungan",
      "created_at": "2026-08-05 11:20:00"
    }
  ]
}
```

> **Catatan:** Field yang tidak dikenali pada skema MySQL akan otomatis diabaikan. Tidak perlu mengirimkan semua kolom — hanya kolom yang tersedia di SQLite desktop.

#### Response — Sukses Penuh (200 OK)

```json
{
  "success": true,
  "message": "Sinkronisasi selesai. 4 baris diproses.",
  "timestamp": "2026-08-05 13:21:45"
}
```

#### Response — Sukses dengan Peringatan (200 OK)

Terjadi jika sebagian record gagal (mis. data rusak / constraint unik) namun sisa data berhasil disimpan:

```json
{
  "success": true,
  "message": "Sinkronisasi selesai. 3 baris diproses.",
  "timestamp": "2026-08-05 13:21:45",
  "warnings": [
    "[journals] SQLSTATE[22001]: String data, right truncated: ..."
  ]
}
```

#### Response — Gagal (500 Internal Server Error)

Terjadi jika terdapat exception fatal di luar blok upsert (mis. gagal query rekonsiliasi):

```json
{
  "success": false,
  "message": "Sinkronisasi gagal: SQLSTATE[HY000]: ..."
}
```

#### Response Error

| HTTP Status | Kondisi |
| :---------- | :------ |
| `400` | Body bukan JSON valid atau bukan array |
| `401` | API Key tidak valid atau tidak dikirimkan |
| `500` | Exception fatal saat rekonsiliasi database |

---

## 🗂️ Tabel yang Didukung untuk Sinkronisasi

Data dua arah (pull & push) didukung pada **18 tabel** berikut, diproses dalam urutan FK-safe:

| # | Nama Tabel | Keterangan |
| :-: | :--------- | :--------- |
| 1 | `academic_years` | Master tahun pelajaran |
| 2 | `classrooms` | Rombongan belajar |
| 3 | `users` | Pengguna sistem & guru/wali kelas |
| 4 | `payment_types` | Jenis tagihan (bulanan / bebas) |
| 5 | `payment_methods` | Metode pembayaran aktif |
| 6 | `students` | Biodata siswa |
| 7 | `payment_assignments` | Tagihan per siswa |
| 8 | `payments` | Kepala transaksi pembayaran |
| 9 | `payment_details` | Rincian item per transaksi |
| 10 | `reliefs` | Keringanan / beasiswa |
| 11 | `journals` | Pembukuan kas masuk/keluar |
| 12 | `savings` | Rekening tabungan siswa |
| 13 | `savings_transactions` | Mutasi setor/tarik tabungan |
| 14 | `debts` | Catatan hutang/piutang lembaga |
| 15 | `settings` | Konfigurasi sistem (PK: `key`) |
| 16 | `student_academic_histories` | Riwayat kenaikan kelas |
| 17 | `online_payment_orders` | Transaksi Midtrans (header) |
| 18 | `online_payment_items` | Rincian transaksi online |

> **Catatan:** Tabel `settings` menggunakan primary key `key` (string), bukan `id` (UUID). Pastikan field `key` selalu disertakan saat push data settings.

---

## 🔄 Alur Sinkronisasi Desktop ↔ Server

```
Desktop Client (SQLite)
        │
        ├─ [1] Pull ──────► GET /sync/pull?since=LAST_SYNC
        │                    Server kirim data terbaru → simpan ke SQLite lokal
        │
        ├─ [2] Kasir bertransaksi offline
        │      (data tersimpan di SQLite desktop)
        │
        └─ [3] Push ──────► POST /sync/push
                             Desktop kirim transaksi baru → server upsert + rekonsiliasi
```

**Strategi timestamp `since`:**
- Simpan `last_sync_at` setiap kali pull berhasil
- Gunakan nilai tersebut sebagai parameter `since` pada pull berikutnya
- Full pull (tanpa `since` atau `since = 1970-01-01 00:00:00`) hanya dilakukan saat instalasi pertama

---

## 🔐 Endpoint Otentikasi Desktop Client & Ping

### 1. `GET /api/v1/ping`
Memeriksa ketersediaan peladen dan informasi sekolah secara publik tanpa autentikasi (digunakan oleh Onboarding Desktop Client).

- **Header:** Tidak memerlukan `X-API-Key`.
- **Response Success (200 OK):**
  ```json
  {
    "success": true,
    "app_name": "Partisipasi Sekolah",
    "school_name": "SMA Negeri 1 Gedeg",
    "version": "1.0.0",
    "server_time": "2026-09-30 12:00:00"
  }
  ```

---

### 2. `POST /api/v1/auth/exchange-code`
Menukarkan Kode Otorisasi (6-digit OTP) yang dibuat oleh peramban menjadi token sesi desktop, profil pengguna, dan *API Key* sekolah (Zero-Config provisioning).

- **Header:** `Content-Type: application/json` (`X-API-Key` opsional)
- **Request Body:**
  ```json
  {
    "code": "482910"
  }
  ```
- **Response Success (200 OK):**
  ```json
  {
    "success": true,
    "message": "Kode otorisasi berhasil diverifikasi.",
    "token": "a1b2c3d4e5f6...64charHex",
    "api_key": "psk_live_a92d4de7b418...",
    "user": {
      "id": "uuid-user-123",
      "username": "kasir_utama",
      "nama": "Siti Aminah",
      "email": "kasir@sekolah.sch.id",
      "role": "kasir"
    }
  }
  ```
- **Keamanan:** Single-use (OTP otomatis hangus setelah dipakai), validitas 10 menit, dan perlindungan rate-limit persisten (maksimal 10 percobaan per 5 menit per IP).

---

### 3. `POST /api/v1/auth/verify-token`
Memvalidasi token otentikasi sesi browser / deep link yang diterima desktop langsung ke server sebelum sesi disimpan ke SQLite lokal, sekaligus mengembalikan *API Key* sekolah.

- **Header:** `Content-Type: application/json` (`X-API-Key` opsional)
- **Request Body:**
  ```json
  {
    "token": "a1b2c3d4e5f6...64charHex"
  }
  ```
- **Response Success (200 OK):**
  ```json
  {
    "success": true,
    "message": "Token terverifikasi.",
    "api_key": "psk_live_a92d4de7b418...",
    "user": {
      "id": "uuid-user-123",
      "username": "kasir_utama",
      "nama": "Siti Aminah",
      "email": "kasir@sekolah.sch.id",
      "role": "kasir"
    }
  }
  ```

---

## 🔒 Catatan Keamanan

- API Key disimpan terenkripsi dan harus dijaga kerahasiaannya
- Endpoint menggunakan `hash_equals()` untuk mencegah timing attack pada perbandingan API Key
- CORS header `Access-Control-Allow-Origin: *` diaktifkan khusus untuk kebutuhan integrasi desktop client native
- Semua halaman login dan redirect desktop menampilkan favicon sekolah untuk mencegah kebingungan phishing

# 📡 OpenAPI 3.0 Specification — RESTful Sync & Student Payment API

Dokumentasi API untuk integrasi aplikasi pendukung dan sinkronisasi data offline-ke-online pada aplikasi Pembayaran Sekolah (Partisipasi Sekolah).

> **Versi API:** `v1`  
> **Format Data:** `JSON`  
> **Header Autentikasi:** `X-API-Key`  
> **Protokol CORS:** Didukung penuh (GET, POST, OPTIONS)

---

## 🔐 Autentikasi & Otorisasi

Seluruh request API wajib menyertakan header berikut:
```http
X-API-Key: psk_live_...
```
*API Key dapat dilihat/diperbarui di menu pengaturan administrator.*

---

## 📌 Daftar Endpoint

### 1. GET `/api/v1/payments/student`

Membaca ringkasan tagihan, rincian item, dan riwayat pembayaran siswa berdasarkan kriteria pencarian unik.

#### Query Parameters

| Parameter | Tipe | Wajib | Keterangan |
| :--- | :--- | :--- | :--- |
| `uuid` | `string` | ❌ * | UUID siswa (opsional jika menggunakan `nisn` atau `nip`) |
| `nisn` | `string` | ❌ * | NISN siswa (opsional jika menggunakan `uuid` atau `nip`) |
| `nip` | `string` | ❌ * | NIP Guru/Tendik (opsional jika mencari data staff) |
| `academic_year_id` | `string` | ❌ | Batasi data tagihan pada T.A tertentu (default: T.A aktif saat ini) |

*\* Catatan: Tepat salah satu dari parameter `uuid`, `nisn`, atau `nip` wajib diisi.*

#### Contoh Response (200 OK)
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
    "generated_at": "2026-08-02T11:18:20+07:00"
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
        "transaction_code": "TX-20260802-001",
        "total_amount": 200000.0,
        "payment_method_name": "Tunai (Kasir)",
        "notes": "Pembayaran bulan Juli",
        "created_at": "2026-08-02 09:30:00",
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

---

### 2. GET `/api/v1/sync/pull`

Mengunduh pembaruan data dari server online untuk seluruh tabel yang terdaftar. Mendukung sinkronisasi inkremental berbasis timestamp untuk efisiensi transfer data.

#### Query Parameters

| Parameter | Tipe | Wajib | Keterangan |
| :--- | :--- | :--- | :--- |
| `since` | `string` | ❌ | Format `YYYY-MM-DD HH:MM:SS`. Jika diisi, server hanya mengembalikan record yang dibuat atau diperbarui sejak waktu tersebut. Jika kosong, mengembalikan seluruh data penuh (full pull). |

#### Contoh Response (200 OK)
```json
{
  "success": true,
  "timestamp": "2026-08-02 11:20:00",
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
    "classrooms": [
      {
        "id": "c-1",
        "name": "X-TKJ-1",
        "academic_year_id": "8d3h-7ns8-29sj-10dm",
        "wali_kelas_user_id": "guru-1",
        "created_at": "2026-03-01 10:00:00",
        "updated_at": null
      }
    ],
    "students": [],
    "payment_types": [],
    "payment_assignments": [],
    "payments": [],
    "payment_details": [],
    "reliefs": [],
    "journals": [],
    "savings": [],
    "savings_transactions": [],
    "debts": [],
    "settings": []
  }
}
```

---

### 3. POST `/api/v1/sync/push`

Mengirim pembaruan/penambahan data dari aplikasi desktop client lokal ke database online. Endpoint ini secara otomatis melakukan kueri **Upsert** (Insert or Update on Duplicate Key) secara transaksional untuk menghindari duplikasi.

#### Request Body (JSON)
Kirimkan dictionary dengan key berupa nama tabel dan value berupa array records. Format data per kolom harus menyesuaikan skema database server.

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
      "created_at": "2026-08-02 11:15:00",
      "updated_at": "2026-08-02 11:15:00"
    }
  ],
  "payment_details": [
    {
      "id": "detail-uuid-999",
      "payment_id": "payment-uuid-999",
      "payment_assignment_id": "90bba468-4355-4c3d-bf84-d4851119ad10",
      "amount": 200000,
      "created_at": "2026-08-02 11:15:00",
      "updated_at": "2026-08-02 11:15:00"
    }
  ]
}
```

#### Contoh Response (200 OK)
```json
{
  "success": true,
  "message": "Sinkronisasi berhasil. 2 baris data diproses.",
  "timestamp": "2026-08-02 11:21:45"
}
```

#### Response Error (400 Bad Request / 500 Internal Server Error)
Jika format JSON salah atau terjadi kegagalan query SQL (misal constraint violation), transaksi database otomatis dibatalkan (rollback) dan mengembalikan respons kegagalan:
```json
{
  "success": false,
  "message": "Gagal melakukan sinkronisasi database: [SQLSTATE Error...]"
}
```

---

## 🗂️ Tabel yang Didukung untuk Sinkronisasi

Aplikasi mendukung pertukaran data dua arah pada 18 tabel berikut:

1. `users` — Pengguna sistem lokal & guru/wali kelas.
2. `academic_years` — Master tahun pelajaran berjalan & riwayat.
3. `classrooms` — Data rombongan belajar.
4. `students` — Biodata siswa & integrasi akun wali.
5. `payment_types` — Jenis tagihan (bebas / bulanan).
6. `payment_methods` — Metode pembayaran yang aktif.
7. `payment_assignments` — Jumlah tagihan per siswa.
8. `payments` — Kepala transaksi pembayaran (header).
9. `payment_details` — Rincian item pembayaran (detail).
10. `reliefs` — Keringanan/diskon yang disetujui.
11. `journals` — Catatan pembukuan keuangan (kas masuk/keluar).
12. `savings` — Rekening tabungan siswa.
13. `savings_transactions` — Mutasi setor/tarik tabungan.
14. `debts` — Catatan utang lembaga.
15. `settings` — Pengaturan konfigurasi sistem.
16. `student_academic_histories` — Riwayat kenaikan kelas/T.A siswa.
17. `online_payment_orders` — Transaksi Midtrans / Pembayaran online (header).
18. `online_payment_items` — Rincian transaksi online.

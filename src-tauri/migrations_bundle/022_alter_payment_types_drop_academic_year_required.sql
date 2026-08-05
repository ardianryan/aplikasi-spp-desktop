-- Pos Bayar (payment_types) tidak lagi wajib memiliki academic_year_id.
-- Tahun pelajaran kini ditentukan oleh assignment (payment_assignments), bukan pos-nya.
-- Pos bayar yang sudah ada tetap memiliki academic_year_id-nya (backward-compatible).

-- 1. Hapus FK constraint
ALTER TABLE `payment_types`
  DROP FOREIGN KEY `fk_payment_types_academic_year`;

-- 2. Set kolom menjadi nullable
ALTER TABLE `payment_types`
  MODIFY `academic_year_id` VARCHAR(36) DEFAULT NULL;

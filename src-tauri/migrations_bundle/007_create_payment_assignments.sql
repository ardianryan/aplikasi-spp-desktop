-- Payment Assignments (Tagihan per Siswa)
CREATE TABLE IF NOT EXISTS `payment_assignments` (
    `id` VARCHAR(36) NOT NULL,
    `student_id` VARCHAR(36) NOT NULL,
    `payment_type_id` VARCHAR(36) NOT NULL,
    `academic_year_id` VARCHAR(36) NOT NULL,
    `month` TINYINT DEFAULT NULL COMMENT '1-12, NULL for bebas type',
    `amount` DECIMAL(15,2) NOT NULL DEFAULT 0,
    `paid_amount` DECIMAL(15,2) NOT NULL DEFAULT 0,
    `relief_amount` DECIMAL(15,2) NOT NULL DEFAULT 0,
    `status` ENUM('belum_bayar','cicilan','lunas','bebas') NOT NULL DEFAULT 'belum_bayar',
    `created_at` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    `updated_at` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (`id`),
    KEY `idx_pa_student` (`student_id`),
    KEY `idx_pa_payment_type` (`payment_type_id`),
    KEY `idx_pa_academic_year` (`academic_year_id`),
    KEY `idx_pa_status` (`status`),
    KEY `idx_pa_month` (`month`),
    CONSTRAINT `fk_pa_student` FOREIGN KEY (`student_id`) REFERENCES `students`(`id`) ON DELETE CASCADE,
    CONSTRAINT `fk_pa_payment_type` FOREIGN KEY (`payment_type_id`) REFERENCES `payment_types`(`id`) ON DELETE CASCADE,
    CONSTRAINT `fk_pa_academic_year` FOREIGN KEY (`academic_year_id`) REFERENCES `academic_years`(`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

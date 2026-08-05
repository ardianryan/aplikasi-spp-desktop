-- Payment Types (Pos Pembayaran)
CREATE TABLE IF NOT EXISTS `payment_types` (
    `id` VARCHAR(36) NOT NULL,
    `name` VARCHAR(100) NOT NULL,
    `type` ENUM('bulanan','bebas') NOT NULL,
    `amount` DECIMAL(15,2) NOT NULL DEFAULT 0,
    `academic_year_id` VARCHAR(36) NOT NULL,
    `description` TEXT DEFAULT NULL,
    `is_active` TINYINT(1) NOT NULL DEFAULT 1,
    `created_at` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (`id`),
    KEY `idx_payment_types_academic_year` (`academic_year_id`),
    KEY `idx_payment_types_type` (`type`),
    CONSTRAINT `fk_payment_types_academic_year` FOREIGN KEY (`academic_year_id`) REFERENCES `academic_years`(`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- Debts (Hutang Piutang)
CREATE TABLE IF NOT EXISTS `debts` (
    `id` VARCHAR(36) NOT NULL,
    `type` ENUM('hutang','piutang') NOT NULL,
    `party_name` VARCHAR(100) NOT NULL,
    `description` TEXT NOT NULL,
    `amount` DECIMAL(15,2) NOT NULL DEFAULT 0,
    `paid_amount` DECIMAL(15,2) NOT NULL DEFAULT 0,
    `status` ENUM('belum_lunas','lunas') NOT NULL DEFAULT 'belum_lunas',
    `due_date` DATE DEFAULT NULL,
    `user_id` VARCHAR(36) NOT NULL,
    `academic_year_id` VARCHAR(36) NOT NULL,
    `created_at` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    `updated_at` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (`id`),
    KEY `idx_debts_type` (`type`),
    KEY `idx_debts_status` (`status`),
    KEY `idx_debts_academic_year` (`academic_year_id`),
    CONSTRAINT `fk_debts_user` FOREIGN KEY (`user_id`) REFERENCES `users`(`id`) ON DELETE CASCADE,
    CONSTRAINT `fk_debts_academic_year` FOREIGN KEY (`academic_year_id`) REFERENCES `academic_years`(`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

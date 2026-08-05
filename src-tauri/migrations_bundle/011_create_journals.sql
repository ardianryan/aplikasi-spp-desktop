-- Journals (Jurnal Umum)
CREATE TABLE IF NOT EXISTS `journals` (
    `id` VARCHAR(36) NOT NULL,
    `date` DATE NOT NULL,
    `type` ENUM('pemasukan','pengeluaran') NOT NULL,
    `category` VARCHAR(100) NOT NULL,
    `description` TEXT NOT NULL,
    `amount` DECIMAL(15,2) NOT NULL DEFAULT 0,
    `reference_id` VARCHAR(36) DEFAULT NULL COMMENT 'Link to payment if from payment',
    `user_id` VARCHAR(36) NOT NULL,
    `academic_year_id` VARCHAR(36) NOT NULL,
    `created_at` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (`id`),
    KEY `idx_journals_date` (`date`),
    KEY `idx_journals_type` (`type`),
    KEY `idx_journals_academic_year` (`academic_year_id`),
    KEY `idx_journals_reference` (`reference_id`),
    CONSTRAINT `fk_journals_user` FOREIGN KEY (`user_id`) REFERENCES `users`(`id`) ON DELETE CASCADE,
    CONSTRAINT `fk_journals_academic_year` FOREIGN KEY (`academic_year_id`) REFERENCES `academic_years`(`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

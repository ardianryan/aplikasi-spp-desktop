-- Savings Transactions
CREATE TABLE IF NOT EXISTS `savings_transactions` (
    `id` VARCHAR(36) NOT NULL,
    `savings_id` VARCHAR(36) NOT NULL,
    `type` ENUM('setor','tarik') NOT NULL,
    `amount` DECIMAL(15,2) NOT NULL DEFAULT 0,
    `balance_after` DECIMAL(15,2) NOT NULL DEFAULT 0,
    `notes` TEXT DEFAULT NULL,
    `user_id` VARCHAR(36) NOT NULL COMMENT 'Petugas',
    `created_at` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (`id`),
    KEY `idx_st_savings` (`savings_id`),
    KEY `idx_st_created_at` (`created_at`),
    CONSTRAINT `fk_st_savings` FOREIGN KEY (`savings_id`) REFERENCES `savings`(`id`) ON DELETE CASCADE,
    CONSTRAINT `fk_st_user` FOREIGN KEY (`user_id`) REFERENCES `users`(`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

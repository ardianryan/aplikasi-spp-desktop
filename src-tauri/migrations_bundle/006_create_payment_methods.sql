-- Payment Methods (Metode Pembayaran Dinamis)
CREATE TABLE IF NOT EXISTS `payment_methods` (
    `id` VARCHAR(36) NOT NULL,
    `name` VARCHAR(100) NOT NULL,
    `type` ENUM('cash','bank_transfer','qris','midtrans') NOT NULL,
    `details` TEXT DEFAULT NULL COMMENT 'JSON: bank info, QRIS payload, etc.',
    `icon` VARCHAR(50) DEFAULT NULL COMMENT 'Material icon name',
    `is_active` TINYINT(1) NOT NULL DEFAULT 1,
    `sort_order` INT NOT NULL DEFAULT 0,
    `created_at` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (`id`),
    KEY `idx_payment_methods_active` (`is_active`, `sort_order`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

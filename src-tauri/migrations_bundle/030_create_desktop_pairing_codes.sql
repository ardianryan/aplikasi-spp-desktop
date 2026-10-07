-- Create desktop_pairing_codes table for quick 6-digit OTP desktop authentication
CREATE TABLE IF NOT EXISTS `desktop_pairing_codes` (
    `id` VARCHAR(36) NOT NULL,
    `user_id` VARCHAR(36) NOT NULL,
    `code` VARCHAR(10) NOT NULL,
    `token` VARCHAR(255) NOT NULL,
    `attempts` INT NOT NULL DEFAULT 0,
    `is_used` TINYINT(1) NOT NULL DEFAULT 0,
    `expires_at` DATETIME NOT NULL,
    `created_at` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (`id`),
    UNIQUE KEY `idx_pairing_code` (`code`),
    KEY `idx_pairing_user_id` (`user_id`),
    CONSTRAINT `fk_pairing_user_id` FOREIGN KEY (`user_id`) REFERENCES `users` (`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

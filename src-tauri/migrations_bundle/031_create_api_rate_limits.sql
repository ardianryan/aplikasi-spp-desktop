-- Create api_rate_limits table for robust brute-force protection
CREATE TABLE IF NOT EXISTS `api_rate_limits` (
    `id` VARCHAR(36) NOT NULL,
    `ip_address` VARCHAR(45) NOT NULL,
    `endpoint` VARCHAR(100) NOT NULL,
    `attempts` INT NOT NULL DEFAULT 1,
    `first_attempt_at` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    `last_attempt_at` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    `blocked_until` DATETIME DEFAULT NULL,
    PRIMARY KEY (`id`),
    UNIQUE KEY `idx_rate_limit_ip_endpoint` (`ip_address`, `endpoint`),
    KEY `idx_rate_limit_blocked` (`blocked_until`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

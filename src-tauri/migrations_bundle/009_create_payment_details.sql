-- Payment Details (Detail per pos per transaksi)
CREATE TABLE IF NOT EXISTS `payment_details` (
    `id` VARCHAR(36) NOT NULL,
    `payment_id` VARCHAR(36) NOT NULL,
    `payment_assignment_id` VARCHAR(36) NOT NULL,
    `amount` DECIMAL(15,2) NOT NULL DEFAULT 0,
    `created_at` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (`id`),
    KEY `idx_pd_payment` (`payment_id`),
    KEY `idx_pd_assignment` (`payment_assignment_id`),
    CONSTRAINT `fk_pd_payment` FOREIGN KEY (`payment_id`) REFERENCES `payments`(`id`) ON DELETE CASCADE,
    CONSTRAINT `fk_pd_assignment` FOREIGN KEY (`payment_assignment_id`) REFERENCES `payment_assignments`(`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

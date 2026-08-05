CREATE TABLE IF NOT EXISTS `online_payment_items` (
    `id` VARCHAR(36) NOT NULL,
    `online_payment_order_id` VARCHAR(36) NOT NULL,
    `payment_assignment_id` VARCHAR(36) NOT NULL,
    `amount` DECIMAL(15,2) NOT NULL DEFAULT 0,
    `created_at` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (`id`),
    KEY `idx_opi_order` (`online_payment_order_id`),
    KEY `idx_opi_assignment` (`payment_assignment_id`),
    CONSTRAINT `fk_opi_order` FOREIGN KEY (`online_payment_order_id`) REFERENCES `online_payment_orders`(`id`) ON DELETE CASCADE,
    CONSTRAINT `fk_opi_assignment` FOREIGN KEY (`payment_assignment_id`) REFERENCES `payment_assignments`(`id`) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

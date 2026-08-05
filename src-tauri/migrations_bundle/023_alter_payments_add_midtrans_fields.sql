ALTER TABLE `payments`
    ADD COLUMN `midtrans_transaction_id` VARCHAR(100) DEFAULT NULL AFTER `midtrans_order_id`,
    ADD COLUMN `midtrans_payment_type` VARCHAR(50) DEFAULT NULL AFTER `midtrans_transaction_id`,
    ADD COLUMN `midtrans_pdf_url` TEXT DEFAULT NULL AFTER `midtrans_payment_type`,
    ADD COLUMN `midtrans_transaction_time` DATETIME DEFAULT NULL AFTER `midtrans_pdf_url`,
    ADD COLUMN `midtrans_raw` TEXT DEFAULT NULL AFTER `midtrans_transaction_time`,
    ADD COLUMN `midtrans_is_applied` TINYINT(1) NOT NULL DEFAULT 0 AFTER `midtrans_status`,
    ADD COLUMN `midtrans_invalidated_at` DATETIME DEFAULT NULL AFTER `midtrans_is_applied`,
    ADD COLUMN `midtrans_invalidated_by` VARCHAR(36) DEFAULT NULL AFTER `midtrans_invalidated_at`,
    ADD KEY `idx_payments_midtrans_order` (`midtrans_order_id`),
    ADD KEY `idx_payments_midtrans_status` (`midtrans_status`),
    ADD KEY `idx_payments_midtrans_invalidated_by` (`midtrans_invalidated_by`),
    ADD CONSTRAINT `fk_payments_midtrans_invalidated_by` FOREIGN KEY (`midtrans_invalidated_by`) REFERENCES `users`(`id`) ON DELETE SET NULL;

-- Reliefs (Keringanan)
CREATE TABLE IF NOT EXISTS `reliefs` (
    `id` VARCHAR(36) NOT NULL,
    `student_id` VARCHAR(36) NOT NULL,
    `payment_assignment_id` VARCHAR(36) NOT NULL,
    `type` ENUM('pembebasan','potongan') NOT NULL,
    `amount` DECIMAL(15,2) NOT NULL DEFAULT 0 COMMENT '0 = pembebasan penuh',
    `reason` TEXT NOT NULL,
    `requested_by` VARCHAR(36) NOT NULL,
    `approved_by` VARCHAR(36) DEFAULT NULL,
    `status` ENUM('pending','approved','rejected') NOT NULL DEFAULT 'pending',
    `approved_at` DATETIME DEFAULT NULL,
    `notes` TEXT DEFAULT NULL COMMENT 'Catatan approval',
    `created_at` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (`id`),
    KEY `idx_reliefs_student` (`student_id`),
    KEY `idx_reliefs_assignment` (`payment_assignment_id`),
    KEY `idx_reliefs_status` (`status`),
    CONSTRAINT `fk_reliefs_student` FOREIGN KEY (`student_id`) REFERENCES `students`(`id`) ON DELETE CASCADE,
    CONSTRAINT `fk_reliefs_assignment` FOREIGN KEY (`payment_assignment_id`) REFERENCES `payment_assignments`(`id`) ON DELETE CASCADE,
    CONSTRAINT `fk_reliefs_requested_by` FOREIGN KEY (`requested_by`) REFERENCES `users`(`id`) ON DELETE CASCADE,
    CONSTRAINT `fk_reliefs_approved_by` FOREIGN KEY (`approved_by`) REFERENCES `users`(`id`) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

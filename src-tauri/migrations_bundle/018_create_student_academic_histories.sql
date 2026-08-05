-- Student Academic Histories
CREATE TABLE IF NOT EXISTS `student_academic_histories` (
    `id` VARCHAR(36) NOT NULL,
    `student_id` VARCHAR(36) NOT NULL,
    `academic_year_id` VARCHAR(36) NOT NULL,
    `classroom_id` VARCHAR(36) DEFAULT NULL,
    `class_name` VARCHAR(50) DEFAULT NULL,
    `status` ENUM('active','alumni','transferred') NOT NULL DEFAULT 'active',
    `synced_at` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    `created_at` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    `updated_at` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (`id`),
    UNIQUE KEY `idx_student_year_unique` (`student_id`, `academic_year_id`),
    KEY `idx_sah_student` (`student_id`),
    KEY `idx_sah_year` (`academic_year_id`),
    KEY `idx_sah_classroom` (`classroom_id`),
    CONSTRAINT `fk_sah_student` FOREIGN KEY (`student_id`) REFERENCES `students`(`id`) ON DELETE CASCADE,
    CONSTRAINT `fk_sah_year` FOREIGN KEY (`academic_year_id`) REFERENCES `academic_years`(`id`) ON DELETE CASCADE,
    CONSTRAINT `fk_sah_classroom` FOREIGN KEY (`classroom_id`) REFERENCES `classrooms`(`id`) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
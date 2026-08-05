-- Classrooms table
CREATE TABLE IF NOT EXISTS `classrooms` (
    `id` VARCHAR(36) NOT NULL,
    `name` VARCHAR(50) NOT NULL,
    `academic_year_id` VARCHAR(36) NOT NULL,
    `wali_kelas_user_id` VARCHAR(36) DEFAULT NULL,
    `created_at` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (`id`),
    KEY `idx_classrooms_academic_year` (`academic_year_id`),
    KEY `idx_classrooms_wali_kelas` (`wali_kelas_user_id`),
    UNIQUE KEY `idx_classrooms_name_year` (`name`, `academic_year_id`),
    CONSTRAINT `fk_classrooms_academic_year` FOREIGN KEY (`academic_year_id`) REFERENCES `academic_years`(`id`) ON DELETE CASCADE,
    CONSTRAINT `fk_classrooms_wali_kelas` FOREIGN KEY (`wali_kelas_user_id`) REFERENCES `users`(`id`) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

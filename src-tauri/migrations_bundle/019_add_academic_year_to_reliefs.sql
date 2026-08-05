-- Add academic_year_id to reliefs table
ALTER TABLE `reliefs`
    ADD COLUMN `academic_year_id` VARCHAR(36) NULL AFTER `id`,
    ADD KEY `idx_reliefs_academic_year` (`academic_year_id`),
    ADD CONSTRAINT `fk_reliefs_academic_year` FOREIGN KEY (`academic_year_id`) REFERENCES `academic_years`(`id`) ON DELETE CASCADE;

-- Populate academic_year_id from students table
UPDATE `reliefs` r
JOIN `students` s ON r.student_id = s.id
SET r.academic_year_id = s.academic_year_id;

-- Make academic_year_id NOT NULL after population
ALTER TABLE `reliefs`
    MODIFY COLUMN `academic_year_id` VARCHAR(36) NOT NULL;

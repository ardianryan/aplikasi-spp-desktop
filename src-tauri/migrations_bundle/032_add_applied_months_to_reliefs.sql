-- Migration: Add applied_months to reliefs table
-- Default NULL means all months (12 months) for backward compatibility
ALTER TABLE `reliefs`
    ADD COLUMN `applied_months` VARCHAR(255) NULL AFTER `academic_year_id`;

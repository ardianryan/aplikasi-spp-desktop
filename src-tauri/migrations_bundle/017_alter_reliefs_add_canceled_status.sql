-- Add canceled status to reliefs
ALTER TABLE `reliefs`
    MODIFY COLUMN `status` ENUM('pending','approved','rejected','canceled') NOT NULL DEFAULT 'pending';

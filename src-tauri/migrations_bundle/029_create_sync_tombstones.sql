-- Migration 029: Create sync_tombstones table for tracking deleted records
CREATE TABLE IF NOT EXISTS `sync_tombstones` (
    `id` VARCHAR(36) NOT NULL,
    `table_name` VARCHAR(64) NOT NULL,
    `record_id` VARCHAR(36) NOT NULL,
    `deleted_at` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (`id`),
    KEY `idx_sync_tombstones_lookup` (`table_name`, `deleted_at`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

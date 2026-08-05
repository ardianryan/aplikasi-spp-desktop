-- Add NIP column to users table for guru/tendik identification
ALTER TABLE `users` ADD COLUMN `nip` VARCHAR(20) DEFAULT NULL AFTER `sso_member_id`;
ALTER TABLE `users` ADD UNIQUE KEY `idx_users_nip` (`nip`);

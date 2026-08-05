-- Allow synced non-student accounts to exist without local role assignment yet.
ALTER TABLE `users`
    MODIFY COLUMN `role` ENUM('admin','bendahara','kasir','wali_kelas','siswa') NULL DEFAULT NULL;

-- Preserve original SSO member role to control assignment rules in local app.
ALTER TABLE `users`
    ADD COLUMN `sso_role` ENUM('siswa','guru','tendik','alumni','keluar') DEFAULT NULL AFTER `nip`;

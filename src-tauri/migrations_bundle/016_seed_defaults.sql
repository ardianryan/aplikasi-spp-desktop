-- Seed: Default admin user and settings
-- Admin login: admin / admin123

INSERT INTO `users` (`id`, `username`, `password`, `nama`, `role`, `is_active`, `created_at`)
VALUES (
    UUID(),
    'admin',
    '$2y$10$SeNvJfbf5M.gCPg5B6mAEOwIK6u0RZxFyr1jPlDkYIDlF6Or3WUJW',
    'Administrator',
    'admin',
    1,
    NOW()
);

-- Default payment method: Tunai
INSERT INTO `payment_methods` (`id`, `name`, `type`, `details`, `icon`, `is_active`, `sort_order`)
VALUES (UUID(), 'Tunai', 'cash', NULL, 'payments', 1, 1);

-- Default settings
INSERT INTO `settings` (`id`, `key`, `value`) VALUES
(UUID(), 'school_name', 'Nama Sekolah'),
(UUID(), 'school_address', 'Alamat Sekolah'),
(UUID(), 'principal_name', 'Nama Kepala Sekolah'),
(UUID(), 'committee_name', 'Nama Komite Sekolah'),
(UUID(), 'logo_left', ''),
(UUID(), 'logo_right', ''),
(UUID(), 'midtrans_client_key', ''),
(UUID(), 'midtrans_server_key', ''),
(UUID(), 'midtrans_is_production', '0'),
(UUID(), 'maintenance_mode', '0'),
(UUID(), 'google_client_id', ''),
(UUID(), 'google_client_secret', ''),
(UUID(), 'google_redirect_uri', ''),
(UUID(), 'sso_base_url', ''),
(UUID(), 'sso_api_key', ''),
(UUID(), 'savings_enabled', '1'),
(UUID(), 'qris_static_payload', ''),
(UUID(), 'qris_api_url', 'https://api-mininxd.vercel.app/qris'),
(UUID(), 'term_pembayaran', 'Pembayaran'),
(UUID(), 'term_tagihan', 'Tagihan'),
(UUID(), 'term_spp', 'SPP'),
(UUID(), 'term_siswa', 'Siswa'),
(UUID(), 'term_sumbangan', 'Sumbangan'),
(UUID(), 'term_iuran', 'Iuran'),
(UUID(), 'term_kwitansi', 'Kwitansi'),
(UUID(), 'term_tunggakan', 'Tunggakan');

-- Increase length of google_avatar to handle long Google URLs
ALTER TABLE `users` MODIFY COLUMN `google_avatar` TEXT DEFAULT NULL;
ALTER TABLE `students` MODIFY COLUMN `google_avatar` TEXT DEFAULT NULL;

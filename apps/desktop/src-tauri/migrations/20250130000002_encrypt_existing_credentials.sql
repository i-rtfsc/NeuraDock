-- Migration: Encrypt existing credentials
-- This migration is designed to be run manually with a backup script
-- because we cannot perform encryption in SQL - it requires Rust code

-- NOTE: This is a PLACEHOLDER migration file
-- The actual encryption of existing data must be done by a Rust migration script
-- that reads all accounts, encrypts cookies and api_user fields, and writes them back

-- Steps to encrypt existing data:
-- 1. Backup the database: cp neuradock.db neuradock.db.backup
-- 2. Run the encryption migration script (see tools/migrate_encrypt_credentials.rs)
-- 3. Verify encrypted data can be decrypted properly
-- 4. Delete backup once confirmed working

-- This file is kept empty to mark that the migration has been considered
-- The actual migration happens in Rust code during application startup
-- if unencrypted data is detected

-- This migration intentionally does nothing to avoid SQL syntax errors
-- Encryption is handled automatically by the backward-compatible decryption logic


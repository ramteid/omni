-- Add must_change_password field to users table
-- This flag forces users to change their password on next login
-- Used for new users created by admins and password resets

ALTER TABLE users
ADD COLUMN must_change_password BOOLEAN NOT NULL DEFAULT FALSE;

-- Create index for efficient filtering
CREATE INDEX idx_users_must_change_password ON users(must_change_password) WHERE must_change_password = TRUE;

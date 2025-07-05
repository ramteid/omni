-- Make password_hash optional for users who use magic links
ALTER TABLE users ALTER COLUMN password_hash DROP NOT NULL;

-- Add columns to track auth method and domain
ALTER TABLE users ADD COLUMN auth_method VARCHAR(50) NOT NULL DEFAULT 'password';
ALTER TABLE users ADD COLUMN domain VARCHAR(255) NULL;

-- Add constraint for auth_method
ALTER TABLE users ADD CONSTRAINT users_auth_method_check 
    CHECK (auth_method IN ('password', 'magic_link', 'both'));

-- Add index for domain-based lookups
CREATE INDEX idx_users_domain ON users(domain);
CREATE INDEX idx_users_auth_method ON users(auth_method);

-- Extract domain from existing user emails
UPDATE users SET domain = SUBSTRING(email FROM '@(.*)$') WHERE domain IS NULL;

-- Add constraint that ensures password_hash is present for password auth
ALTER TABLE users ADD CONSTRAINT users_password_hash_check 
    CHECK (
        (auth_method = 'password' AND password_hash IS NOT NULL) OR
        (auth_method = 'magic_link' AND password_hash IS NULL) OR
        (auth_method = 'both' AND password_hash IS NOT NULL)
    );
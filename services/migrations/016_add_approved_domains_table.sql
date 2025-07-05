-- Add approved_domains table for domain-based auto-registration
CREATE TABLE approved_domains (
    id CHAR(26) PRIMARY KEY,
    domain VARCHAR(255) NOT NULL UNIQUE,
    approved_by CHAR(26) NOT NULL REFERENCES users(id),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Index for fast domain lookups
CREATE INDEX idx_approved_domains_domain ON approved_domains(domain);
CREATE INDEX idx_approved_domains_approved_by ON approved_domains(approved_by);

-- Add trigger to update updated_at
CREATE OR REPLACE FUNCTION update_approved_domains_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER approved_domains_updated_at_trigger
    BEFORE UPDATE ON approved_domains
    FOR EACH ROW
    EXECUTE FUNCTION update_approved_domains_updated_at();
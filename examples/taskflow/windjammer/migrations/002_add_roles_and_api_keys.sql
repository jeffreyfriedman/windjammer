-- Add roles and API keys support

-- Add role column to users table
ALTER TABLE users ADD COLUMN role VARCHAR(20) NOT NULL DEFAULT 'member';
CREATE INDEX idx_users_role ON users(role);

-- Create API keys table
CREATE TABLE api_keys (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    key_hash VARCHAR(64) NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    last_used_at BIGINT,
    created_at BIGINT NOT NULL,
    expires_at BIGINT,
    revoked BOOLEAN NOT NULL DEFAULT false
);

CREATE INDEX idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX idx_api_keys_key_hash ON api_keys(key_hash);
CREATE INDEX idx_api_keys_revoked ON api_keys(revoked) WHERE revoked = false;

-- Add soft delete columns to tasks table
ALTER TABLE tasks ADD COLUMN deleted_at BIGINT;
ALTER TABLE tasks ADD COLUMN deleted_by INTEGER REFERENCES users(id);
CREATE INDEX idx_tasks_deleted_at ON tasks(deleted_at) WHERE deleted_at IS NOT NULL;

-- Add soft delete columns to projects table
ALTER TABLE projects ADD COLUMN deleted_at BIGINT;
ALTER TABLE projects ADD COLUMN deleted_by INTEGER REFERENCES users(id);
CREATE INDEX idx_projects_deleted_at ON projects(deleted_at) WHERE deleted_at IS NOT NULL;

-- Create audit log table
CREATE TABLE audit_log (
    id SERIAL PRIMARY KEY,
    user_id INTEGER REFERENCES users(id),
    action VARCHAR(100) NOT NULL,
    resource_type VARCHAR(50) NOT NULL,
    resource_id INTEGER,
    ip_address VARCHAR(45),
    user_agent TEXT,
    request_id VARCHAR(100),
    metadata JSONB,
    created_at BIGINT NOT NULL
);

CREATE INDEX idx_audit_log_user_id ON audit_log(user_id);
CREATE INDEX idx_audit_log_action ON audit_log(action);
CREATE INDEX idx_audit_log_resource ON audit_log(resource_type, resource_id);
CREATE INDEX idx_audit_log_created_at ON audit_log(created_at);
CREATE INDEX idx_audit_log_request_id ON audit_log(request_id);

-- Update existing users to have admin role for first user
UPDATE users SET role = 'admin' WHERE id = 1;


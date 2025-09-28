-- Create chats table to store chat sessions
CREATE TABLE chats (
    id CHAR(26) PRIMARY KEY,
    user_id CHAR(26) NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for efficient queries by user
CREATE INDEX idx_chats_user_id ON chats(user_id);

-- Index for sorting by creation time
CREATE INDEX idx_chats_created_at ON chats(created_at DESC);

-- Index for sorting by update time
CREATE INDEX idx_chats_updated_at ON chats(updated_at DESC);

-- Trigger to update updated_at timestamp
CREATE TRIGGER update_chats_updated_at
    BEFORE UPDATE ON chats
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Create chat_messages table to store chat messages
CREATE TABLE chat_messages (
    id CHAR(26) PRIMARY KEY,
    chat_id CHAR(26) NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    message_seq_num INTEGER NOT NULL,
    message JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(chat_id, message_seq_num)
);

-- Index for efficient queries by chat
CREATE INDEX idx_chat_messages_chat_id ON chat_messages(chat_id);

-- Index for sorting by sequence number within a chat
CREATE INDEX idx_chat_messages_chat_seq ON chat_messages(chat_id, message_seq_num);

-- Index for JSONB message queries if needed
CREATE INDEX idx_chat_messages_message ON chat_messages USING GIN(message);
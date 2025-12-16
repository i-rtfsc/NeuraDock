-- Add missing API path fields to providers table
ALTER TABLE providers ADD COLUMN token_api_path TEXT;
ALTER TABLE providers ADD COLUMN models_path TEXT;

-- Update existing builtin providers with their API paths
UPDATE providers SET token_api_path = '/api/token/' WHERE id = 'anyrouter';
UPDATE providers SET models_path = '/api/user/models' WHERE id = 'anyrouter';
UPDATE providers SET token_api_path = '/api/token/' WHERE id = 'agentrouter';
UPDATE providers SET models_path = '/api/user/models' WHERE id = 'agentrouter';

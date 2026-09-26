-- Runs once, when the database volume is created. The `test` role can only
-- touch `kosmotheka_test`, so the credentials in .env.test are not a secret.
CREATE ROLE test LOGIN PASSWORD 'test';
CREATE DATABASE kosmotheka_test OWNER test;

-- Runs once, when the database volume is created. The `test` role can only
-- touch `library_test`, so the credentials in .env.test are not a secret.
CREATE ROLE test LOGIN PASSWORD 'test';
CREATE DATABASE library_test OWNER test;

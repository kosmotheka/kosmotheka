-- A library is founded by one key (D97): its id derives from the public key of the device that founded it and
-- this nonce (16 random bytes, base64url without padding), written by the founding operation and never changed.
-- Null for a library nobody founded; such a library has no roles and nobody may write in it.
ALTER TABLE libraries ADD COLUMN founding_nonce TEXT;

-- The seal is gone (D94 is replaced by D97): a library has roles from its first operation, so there is no step
-- from "no roles" to "roles" for a seal to fence. Fences stay (D84, D87).
DROP TABLE device_seals;

-- The library every installation had before D85, with a fixed id, is gone: a library is founded now (D97) and the
-- ones from before are not converted. The first library of a new database is founded when it first starts.
DELETE FROM libraries WHERE id = '00000000-0000-7000-8000-000000000001';

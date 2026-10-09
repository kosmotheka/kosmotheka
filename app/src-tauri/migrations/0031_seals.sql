-- A seal (D94) is an applied operation that fences the device of its own author:
-- it fences every device of the library, named or not. A device a seal does not
-- name has no operation it believes dated before it.
--
-- Derived from the log, like `device_fences`: it is cleared and rebuilt with the
-- library. The seal's named devices are the rows of `device_fences` with the same
-- `operation_id`. The schema version rises with this migration because a device
-- without the rule would judge differently and must not sync with one that has it.
CREATE TABLE device_seals (
    library_id TEXT NOT NULL,
    operation_id TEXT NOT NULL,
    until_hlc TEXT NOT NULL,
    PRIMARY KEY (library_id, operation_id)
) STRICT, WITHOUT ROWID;

-- Fences already noted: those an operation sets on its own author's device.
INSERT INTO device_seals (library_id, operation_id, until_hlc)
SELECT f.library_id, f.operation_id, f.until_hlc
  FROM device_fences f
  JOIN operations o ON o.library_id = f.library_id AND o.id = f.operation_id
 WHERE o.device_id = f.device_id;

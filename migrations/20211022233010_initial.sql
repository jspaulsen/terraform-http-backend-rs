CREATE TABLE IF NOT EXISTS terraform (
    id TEXT PRIMARY KEY,
    state TEXT NOT NULL,
    last_update_ts datetime NOT NULL DEFAULT current_timestamp
);

CREATE TABLE IF NOT EXISTS locks (
    id TEXT NOT NULL,
    terraform_id TEXT PRIMARY KEY,
    state TEXT NOT NULL,
    last_update_ts datetime NOT NULL DEFAULT current_timestamp
);



CREATE TRIGGER IF NOT EXISTS update_resource_ts
AFTER UPDATE ON terraform
BEGIN
    UPDATE terraform SET last_update_ts = current_timestamp WHERE id = NEW.id;
END;


CREATE TRIGGER IF NOT EXISTS update_resource_ts
AFTER UPDATE ON terraform
BEGIN
    UPDATE terraform SET last_update_ts = current_timestamp WHERE id = NEW.id;
END;

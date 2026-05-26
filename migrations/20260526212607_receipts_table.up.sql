CREATE TABLE
    receipts (
        id uuid NOT NULL CONSTRAINT receipts_pk PRIMARY KEY,
        description VARCHAR(255),
        datetime TIMESTAMP
    );
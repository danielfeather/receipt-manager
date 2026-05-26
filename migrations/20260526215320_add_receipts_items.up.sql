CREATE TABLE
    receipt_items (
        receipt_id uuid NOT NULL CONSTRAINT receipt_items_receipts_id_fk REFERENCES receipts,
        index INTEGER NOT NULL,
        description VARCHAR(128),
        quantity INT DEFAULT 1 NOT NULL,
        amount INTEGER NOT NULL,
        CONSTRAINT table_name_pk PRIMARY KEY (receipt_id, index)
    );
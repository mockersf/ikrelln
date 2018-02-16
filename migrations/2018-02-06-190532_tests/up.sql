CREATE TABLE test_item
(
    id VARCHAR(36) NOT NULL PRIMARY KEY,
    parent_id VARCHAR(36),
    name VARCHAR(255) NOT NULL,
    source INT NOT NULL
);
CREATE TABLE test_result
(
    test_id VARCHAR NOT NULL,
    trace_id VARCHAR(36) NOT NULL,
    date TIMESTAMP NOT NULL,
    status INT NOT NULL,
    duration BIGINT NOT NULL,
    environment VARCHAR,
    PRIMARY KEY (test_id, trace_id),
    FOREIGN KEY (test_id) REFERENCES test_item(id)
);

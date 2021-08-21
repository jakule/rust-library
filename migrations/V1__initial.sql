CREATE TABLE IF NOT EXISTS books
(
    id SERIAL PRIMARY KEY NOT NULL,
    name VARCHAR(255),
    author VARCHAR(255),
    publication_year integer,
    created_at timestamp with time zone DEFAULT (now() at time zone 'utc')
);

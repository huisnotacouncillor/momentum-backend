CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL
);
INSERT INTO users (name) VALUES ('测试用户');

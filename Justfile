default:
    @just --list

[confirm]
reset_database:
    sqlx database reset --source database/migrations/
    -rm database/file_uploads/*

pub fn is_foreign_key_error(error: &sqlx::Error) -> bool {
    match error {
        sqlx::Error::Database(error) => matches!(error.message(), "FOREIGN KEY constraint failed"),
        _ => false,
    }
}

//! The flight recorder: every run leaves a queryable record in one SQLite file.

#[cfg(test)]
mod tests {
    /// The bundled SQLite builds and works: open in memory, create, insert, read.
    #[test]
    fn sqlite_opens_and_stores_a_row() {
        let db = rusqlite::Connection::open_in_memory().unwrap();
        db.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT)", []).unwrap();
        db.execute("INSERT INTO t (name) VALUES (?1)", ["hello"]).unwrap();
        let name: String = db.query_row("SELECT name FROM t WHERE id = 1", [], |row| row.get(0)).unwrap();
        assert_eq!(name, "hello");
    }
}

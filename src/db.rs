use rusqlite::{Connection, Result};

pub fn init_db(path: &str) -> Result<Connection, rusqlite::Error> {
    let con = Connection::open(path)?;

    con.execute(
        "create table if not exists posts (
        id integer primary key autoincrement,
        content text not null,
        
        user text not null,
        avatar_url text,
      
        thumbnail_url text,
      
        created_at text not null default current_timestamp
      )",
        [],
    )?;

    Ok(con)
}

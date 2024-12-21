//! This module contains all the functions that are used to interact with the database
//! The database is used to store the files that are uploaded

use std::str::FromStr;

use chrono::{DateTime, Local};
use colored::Colorize;
use rusqlite::Connection;
use sea_query::{ColumnDef, Expr, Iden, Order, Query, SqliteQueryBuilder, Table};
use tabled::Tabled;

/// Establishes a connection to the database
/// The database name is specified in the DB_NAME constant
pub fn connect_to_db(path: &str) -> Result<Connection, rusqlite::Error> {
    Connection::open(path)
}

#[derive(Iden)]
enum Store {
    Table,
    Id,
    Name,
    Path,
    ProjectName,
    Language,
    Preserve,
    CreatedAt,
}

/// Represents a Database Entry
/// It directly reflects the database
#[derive(Debug, Tabled, Clone)]
pub struct Entry {
    pub id: i32,
    pub name: String,
    pub path: String,
    pub project_name: String,
    pub language: String,
    pub preserve: String,
    pub created_at: DateTime<Local>,
}

/// Builder struct that converts to an Entry
#[derive(Debug, Clone)]
pub struct EntryBuilder {
    pub name: String,
    pub path: String,
    pub project_name: String,
    pub language: String,
    pub preserve: Vec<String>,
}

impl EntryBuilder {
    pub fn new(
        name: &str,
        path: &str,
        project_name: &str,
        language: &str,
        preserve: Option<Vec<String>>,
    ) -> Self {
        Self {
            name: name.to_string(),
            path: path.to_string(),
            project_name: project_name.to_string(),
            language: language.to_string(),
            preserve: preserve.unwrap_or(vec![]).to_vec(),
        }
    }
}

/// Prepares the Database, creates all the tables and defines the schema
pub fn prep_db(conn: &Connection) -> rusqlite::Result<usize, rusqlite::Error> {
    let query = Table::create()
        .table(Store::Table)
        .if_not_exists()
        .col(
            ColumnDef::new(Store::Id)
                .integer()
                .not_null()
                .auto_increment()
                .primary_key(),
        )
        .col(ColumnDef::new(Store::Name).string().not_null())
        .col(ColumnDef::new(Store::Path).string().not_null())
        .col(ColumnDef::new(Store::ProjectName).string().not_null())
        .col(ColumnDef::new(Store::Language).string().not_null())
        .col(ColumnDef::new(Store::Preserve).string().not_null())
        .col(ColumnDef::new(Store::CreatedAt).date_time().not_null())
        .build(SqliteQueryBuilder);

    conn.execute(&query, [])
}

fn compress_vec(v: &Vec<String>) -> String {
    v.iter().fold(String::new(), |mut acc, f| {
        acc.push_str(f);
        acc
    })
}

fn decompress_to_vec(v: String) -> Vec<String> {
    v.split(",").map(|f| (f.to_string())).collect()
}

/// Inserts an entry into the database
///
/// # Arguments
///
/// * `conn` - A reference to the database connection
/// * `eb` - An EntryBuilder struct
///
/// # Returns
/// A Result enum with the following variants:
///
/// * `Entry` - The entry that was inserted into the database
/// * `rusqlite::Error` - The error that was encountered while inserting into the database
pub fn insert_into_db(conn: &Connection, eb: EntryBuilder) -> Result<Entry, rusqlite::Error> {
    let time_now = Local::now().to_string();

    let query = Query::insert()
        .into_table(Store::Table)
        .columns([
            Store::Name,
            Store::Path,
            Store::ProjectName,
            Store::Language,
            Store::Preserve,
            Store::CreatedAt,
        ])
        .values_panic([
            eb.name.clone().into(),
            eb.path.clone().into(),
            eb.project_name.clone().into(),
            eb.language.clone().into(),
            compress_vec(&eb.preserve).clone().into(),
            time_now.into(),
        ])
        .to_string(SqliteQueryBuilder);

    match does_exist(conn, &eb.path) {
        Ok(entry) => {
            if delete_entry(conn, &eb.path).is_ok() {
                println!("Updated entry: {}", entry.path.blue());
            } else{
                println!("Failed to update entry: {}", entry.path);
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => {}
        Err(_) => {}
    }

    let _ = conn.execute(&query, []);

    does_exist(conn, &eb.path)
}

/// Gets all the entries from the database
///
/// # Arguments
///
/// * `conn` - A reference to the database connection
///
/// # Returns
/// A Result enum with the following variants:
///
/// * `Vec<Entry>` - A vector of all the entries in the database
/// * `rusqlite::Error` - The error that was encountered while getting the entries from the database
pub fn get_all(conn: &Connection) -> Result<Vec<Entry>, rusqlite::Error> {
    let query = Query::select()
        .columns([
            Store::Id,
            Store::Name,
            Store::Path,
            Store::ProjectName,
            Store::Language,
            Store::Preserve,
            Store::CreatedAt,
        ])
        .order_by(Store::Id, Order::Desc)
        .from(Store::Table)
        .to_string(SqliteQueryBuilder);

    let mut stmt = conn.prepare(&query)?;

    let entries = stmt
        .query_map([], |row| {
            let created_at = chrono::DateTime::from_str(row.get::<_, String>(6)?.as_str())
                .unwrap_or(Local::now());

            Ok(Entry {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                project_name: row.get(3)?,
                language: row.get(4)?,
                preserve: row.get(5)?,
                created_at,
            })
        })?
        .map(|x| x.unwrap())
        .collect::<Vec<Entry>>();

    Ok(entries)
}

/// Gets an entry from the database
/// using the path of the file
/// essentially checking if the file exists
/// in the database
///
/// # Arguments
///
/// * `conn` - A reference to the database connection
/// * `path` - The path of the file
///
/// # Returns
///
/// A Result enum with the following variants:
///
/// * `Entry` - The entry that was found in the database
/// * `rusqlite::Error` - The error that was encountered while getting the entry from the database
///
/// # Usage
///
/// To essentially check if an entry exists, the Error `rusqlite::Error::QueryReturnedNoRows` is
/// returned if the entry does not exist in the database
/// Otherwise, the entry can be essentially used as a normal entry
pub fn does_exist(conn: &Connection, path: &str) -> Result<Entry, rusqlite::Error> {
    let query = Query::select()
        .columns([
            Store::Id,
            Store::Name,
            Store::Path,
            Store::ProjectName,
            Store::Language,
            Store::Preserve,
            Store::CreatedAt,
        ])
        .from(Store::Table)
        .and_where(Expr::col(Store::Path).eq(path))
        .limit(1)
        .to_string(SqliteQueryBuilder);

    conn.query_row(&query, [], |row| {
        let created_at =
            chrono::DateTime::from_str(row.get::<_, String>(6)?.as_str()).unwrap_or(Local::now());

        Ok(Entry {
            id: row.get(0)?,
            name: row.get(1)?,
            path: row.get(2)?,
            project_name: row.get(3)?,
            language: row.get(4)?,
            preserve: row.get(5)?,
            created_at,
        })
    })
}

/// Delete an entry from the database
/// using the path of the file
///
/// # Arguments
///
/// * `conn` - A reference to the database connection
/// * `path` - The path of the file
///
/// # Returns
///
/// A Result enum with the following variants:
///
/// * `usize` - The number of rows that were deleted
/// * `rusqlite::Error` - The error that was encountered while deleting the entry from the database
pub fn delete_entry(conn: &Connection, path: &str) -> Result<usize, rusqlite::Error> {
    let query = Query::delete()
        .from_table(Store::Table)
        .and_where(Expr::col(Store::Path).eq(path))
        .to_string(SqliteQueryBuilder);

    conn.execute(&query, [])
}

/// Delete all the entries from the database
/// Basically, it drops the table
///
/// Better than deleting all the entries one by one
///
/// # Arguments
///
/// * `conn` - A reference to the database connection
///
/// # Returns
///
/// A Result enum with the following variants:
///
/// * `usize` - The number of rows that were deleted
/// * `rusqlite::Error` - The error that was encountered while deleting the entries from the database
pub fn delete_all(conn: &Connection) -> Result<usize, rusqlite::Error> {
    let table_del = Table::truncate()
        .table(Store::Table)
        .to_string(SqliteQueryBuilder);

    conn.execute(&table_del, [])
}

use std::{path::PathBuf, str::FromStr};

use clap::{command, Parser};
use colored::Colorize;
use db::{Entry, EntryBuilder};
use tabled::{settings::Style, Table};

mod db;
mod file;
mod handler;

#[derive(Parser, Debug, Clone)]
#[command(name="garman", author="Ishan Joshi <noobscience@duck.com>", version, about="Gargae Man for all your programs", long_about = None)]
struct Args {
    #[arg(help = "The command to be executed, use --help for more information")]
    cmd: String,

    #[arg(help = "Specify the path")]
    target: Option<Vec<String>>,

    #[arg(short, long, help = "Specify the project path")]
    project: Option<String>,

    #[arg(long, help = "Paths to preserve in the dir")]
    preserve: Option<String>,

    #[arg(long, help = "The patterns to be used for scanning")]
    patterns: Option<String>,

    #[arg(short, long, help = "Specify the language")]
    lang: Option<String>,
}

const COMMANDS: [&str; 5] = ["add", "clean", "list", "show", "delete"];

fn main() {
    let args = Args::parse();
    let cmd = args.cmd;

    if !COMMANDS.contains(&cmd.as_str()) {
        println!("Invalid Command: {}", cmd.red());
        return;
    }

    let paths = args.target.unwrap_or(vec![".".to_string()]);
    let canon_paths = paths
        .iter()
        .map(|x| {
            PathBuf::from_str(x)
                .unwrap()
                .canonicalize()
                .unwrap()
                .to_string_lossy()
                .to_string()
        })
        .collect::<Vec<String>>();

    file::check_paths_exist();

    let compiled_paths: Vec<(String, String, String)> = paths
        .iter()
        .map(|x| {
            let path = PathBuf::from_str(x).unwrap();
            let full_path = path.canonicalize().unwrap().to_string_lossy().to_string();
            let path_name = path
                .canonicalize()
                .unwrap()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();
            (x.to_string(), full_path, path_name)
        })
        .collect();
    let project = args.project.unwrap_or(compiled_paths[0].2.clone());

    //TODO: Add lang prediction
    let lang = args.lang.unwrap_or("text".to_string());
    let mut preserve: Option<Vec<String>> = None;
    if let Some(p) = args.preserve {
        preserve = Some(p.split(",").map(|x| (x.to_string())).collect())
    }

    let db_path = file::get_path("store.db").to_string_lossy().to_string();
    let conn = db::connect_to_db(&db_path).expect("Unable to connect to DB");

    db::prep_db(&conn).expect("Unable to init db");

    match cmd.as_str() {
        "add" => {
            for path in compiled_paths {
                let eb = EntryBuilder::new(
                    &path.2,
                    &path.1,
                    &project.clone(),
                    &lang.clone(),
                    preserve.clone(),
                );

                match db::insert_into_db(&conn, eb) {
                    Ok(entry) => {
                        println!("Added entry: {}", entry.name.green());
                    }
                    Err(_) => {
                        println!("Failed to add entry: {}", path.1);
                    }
                }
            }
        }

        "show" => {
            if canon_paths.len() > 1 {
                println!("Constructing table");
                if let Ok(all) = db::get_all(&conn) {
                    let filtered: Vec<Entry> = all
                        .iter()
                        .filter(|x| canon_paths.contains(&x.path))
                        .cloned()
                        .collect();
                    let table = Table::new(filtered)
                        .with(Style::modern_rounded())
                        .to_string();
                    println!("{}", table);
                }
            } else if let Ok(entry) = db::does_exist(&conn, &canon_paths[0]) {
                println!("Name: {}", entry.name.blue());
                println!("Path: {}", entry.path.yellow());
                println!("Project: {}", entry.project_name.blue());
                println!("Language: {}", entry.language.green());
                println!("Preserve: {}", entry.preserve);
                println!("Created At: {}", entry.created_at.to_string().purple());
            } else {
                println!("No entry found for path: {}", paths[0]);
            }
        }

        "list" => {
            if let Ok(all) = db::get_all(&conn) {
                let table = Table::new(all).with(Style::modern_rounded()).to_string();
                println!("{}", table);
            }
        }

        "delete" => {
            for path in canon_paths {
                if let Ok(entry) = db::does_exist(&conn, &path) {
                    if db::delete_entry(&conn, &path).is_ok() {
                        println!("Deleted entry: {}", entry.path.red());
                    } else {
                        println!("Failed to delete entry: {}", entry.path);
                    }
                } else {
                    println!("No entry found for path: {}", path);
                }
            }
        }

        _ => {
            todo!();
        }
    }
}

use csv::{ReaderBuilder, StringRecord};
use std::{
    env::{self, VarError},
    error::{self},
    fs::{self, File},
    path::Path,
    process::{Command, Stdio},
};
use tracing::{info, warn};

pub fn insert_data(generated_sql_path: String) -> Result<(), Box<dyn std::error::Error>> {
    let generated_sql_file = File::open(generated_sql_path.clone())?;

    let disable_memcached_flush: bool = match env::var("DISABLE_MEMCACHED_FLUSH") {
        Ok(s) => s.parse()?,
        Err(VarError::NotPresent) => false,
        Err(VarError::NotUnicode(_)) => {
            panic!("$DISABLE_MEMCACHED_FLUSH should be written in Unicode.")
        }
    };

    Command::new("mysql")
        .arg(format!("-u{}", env::var("MYSQL_USER").unwrap()))
        .arg(format!("-p{}", env::var("MYSQL_PASSWORD").unwrap()))
        .arg(format!("-h{}", env::var("MYSQL_HOST").unwrap()))
        .arg("--default-character-set=utf8mb4")
        .arg("-e")
        .arg(format!(
            "CREATE DATABASE IF NOT EXISTS {}",
            env::var("MYSQL_DATABASE").unwrap()
        ))
        .spawn()
        .expect("Failed to create database.")
        .wait()?;

    Command::new("mysql")
        .arg(format!("-u{}", env::var("MYSQL_USER").unwrap()))
        .arg(format!("-p{}", env::var("MYSQL_PASSWORD").unwrap()))
        .arg(format!("-h{}", env::var("MYSQL_HOST").unwrap()))
        .arg("--default-character-set=utf8mb4")
        .arg(env::var("MYSQL_DATABASE").unwrap())
        .stdin(Stdio::from(generated_sql_file))
        .spawn()
        .expect("Failed to insert.")
        .wait()?;

    if !disable_memcached_flush {
        let memcached_url = env::var("MEMCACHED_URL")?;
        let cache_client = memcache::connect(memcached_url)?;
        cache_client.flush()?;
    }

    Ok(())
}

pub fn generate_sql() -> Result<String, Box<dyn std::error::Error>> {
    let out_path = match env::var("SQL_OUT_PATH") {
        Ok(s) => s,
        Err(VarError::NotPresent) => "./out.sql".to_string(),
        Err(VarError::NotUnicode(_)) => panic!("$SQL_OUT_PATH should be written in Unicode."),
    };

    let data_path = Path::new("data");

    let entries = fs::read_dir(data_path).expect("The `data` directory could not be found.");
    let mut file_list: Vec<_> = entries
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.is_file() && path.extension()? == "csv" {
                Some(path.file_name()?.to_string_lossy().into_owned())
            } else {
                None
            }
        })
        .collect();
    file_list.sort();

    let mut sql_lines = Vec::new();

    for file_name in &file_list {
        if file_name
            .split('!')
            .nth(1)
            .unwrap_or_default()
            .split('.')
            .nth(1)
            .unwrap_or_default()
            != "csv"
        {
            continue;
        }

        let mut rdr = ReaderBuilder::new().from_path(data_path.join(file_name))?;
        let headers_record = rdr.headers()?;
        let headers: Vec<String> = headers_record
            .into_iter()
            .map(|row| row.to_string())
            .collect();

        let mut csv_data: Vec<StringRecord> = Vec::new();
        let mut rdr = ReaderBuilder::new().from_path(data_path.join(file_name))?;
        let records: Vec<StringRecord> = rdr.records().filter_map(|row| row.ok()).collect();
        csv_data.extend(records);

        let table_name = file_name
            .split('!')
            .nth(1)
            .unwrap_or_default()
            .split('.')
            .next()
            .unwrap_or_default();

        let mut sql_lines_inner = Vec::new();
        sql_lines_inner.push(format!(
            "LOCK TABLES `{}` WRITE;\nINSERT INTO `{}` VALUES ",
            table_name, table_name
        ));

        for (idx, data) in csv_data.iter().enumerate() {
            let cols: Vec<_> = data
                .iter()
                .enumerate()
                .filter_map(|(col_idx, col)| {
                    if headers
                        .get(col_idx)
                        .unwrap_or(&String::new())
                        .starts_with('#')
                    {
                        return None;
                    }

                    if col.is_empty() {
                        Some("NULL".to_string())
                    } else {
                        Some(format!("'{}'", col.replace('\'', "\\'")))
                    }
                })
                .collect();

            sql_lines_inner.push(if idx == csv_data.len() - 1 {
                format!("({});", cols.join(","))
            } else {
                format!("({}),", cols.join(","))
            });
        }

        sql_lines.push(format!("{}\nUNLOCK TABLES", sql_lines_inner.concat()));
    }

    let create_sql: String =
        String::from_utf8_lossy(&fs::read(data_path.join("create_table.sql"))?).parse()?;

    fs::write(
        out_path.clone(),
        format!("{}{};", create_sql, sql_lines.join(";")),
    )?;

    Ok(out_path)
}

fn main() -> Result<(), Box<dyn error::Error>> {
    tracing_subscriber::fmt::init();
    if dotenv::from_filename(".env.local").is_err() {
        warn!("Could not load .env.local");
    };

    let generated_sql_path = generate_sql()?;
    insert_data(generated_sql_path)?;

    info!("Migration successfully completed!");

    Ok(())
}

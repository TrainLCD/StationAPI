use csv::{ReaderBuilder, StringRecord};
use std::{
    env::{self, VarError},
    error,
    fs::{self},
    io::{self, Write},
    path::Path,
    process::Command,
};
use tracing::{info, warn};

pub fn insert_data(generated_sql_path: String) -> Result<(), Box<dyn std::error::Error>> {
    // let output = Command::new("psql")
    //     .arg(format!("-h{}", env::var("POSTGRES_HOST")?))
    //     .arg(format!("-p{}", env::var("POSTGRES_PORT")?))
    //     .arg(format!("-U{}", env::var("POSTGRES_USER")?))
    //     .arg("postgres")
    //     .arg("--command")
    //     .arg("CREATE COLLATION ignore_accents (provider = icu, locale = 'und-u-ks-level1-kc-true', deterministic = false);")
    //     .output()?;

    // io::stdout().write_all(&output.stdout)?;
    // io::stderr().write_all(&output.stderr)?;

    let output = Command::new("psql")
        .arg(format!("-h{}", env::var("POSTGRES_HOST")?))
        .arg(format!("-p{}", env::var("POSTGRES_PORT")?))
        .arg(format!("-U{}", env::var("POSTGRES_USER")?))
        .arg(env::var("POSTGRES_DB")?)
        .arg(format!("-f{}", generated_sql_path))
        .output()?;

    io::stdout().write_all(&output.stdout)?;
    io::stderr().write_all(&output.stderr)?;

    assert!(output.status.success());

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

    let mut sql_lines: Vec<String> = Vec::new();

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

        let mut sql_lines_inner = vec!["\n".to_string()];
        sql_lines_inner.push(format!(
            "\nCOPY stationapi.{} ({}) FROM stdin;\n",
            table_name,
            headers
                .clone()
                .into_iter()
                .filter(|h| !h.starts_with('#'))
                .collect::<Vec<String>>()
                .join(", ")
        ));

        for data in csv_data.iter() {
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
                        Some("\\N".to_string())
                    } else {
                        Some(col.replace('\n', "\\n").to_string())
                    }
                })
                .collect();

            sql_lines_inner.push(cols.join("\t").to_string());
            sql_lines_inner.push("\n".to_string());
        }

        sql_lines.push(format!("{}\\.\n", sql_lines_inner.concat()));
    }

    let create_sql: String =
        String::from_utf8_lossy(&fs::read(data_path.join("create_table.sql"))?).parse()?;

    fs::write(
        out_path.clone(),
        format!("{}{}", create_sql, sql_lines.join("\n")),
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

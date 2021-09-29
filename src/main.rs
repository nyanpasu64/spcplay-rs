use std::fs::create_dir_all;
use std::path::Path;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use rusqlite::Connection;

#[derive(Debug)]
struct Person {
    id: i32,
    name: String,
    data: Option<Vec<u8>>,
}

static SETTINGS_NAME: &str = "settings.sqlite3";

fn main() -> Result<()> {
    // On windows, dirs's ProjectDirs::from("org", "username", "appname") creates the
    // path "username/appname". See https://github.com/dirs-dev/directories-rs/blob/main/src/win.rs#L94.
    // Does anyone actually like the extra layer of path?
    // In my experience, all it does is make it harder to find the folder corresponding
    // to an app. So I'm omitting the second argument.
    let proj_dirs =
        ProjectDirs::from("", "", "spcplay-rs").context("failed to locate config file dir")?;

    let config_dir: &Path = proj_dirs.config_dir();
    create_dir_all(config_dir).context("creating config file dir")?;

    let settings_path = config_dir.join(SETTINGS_NAME);

    let conn = Connection::open(&settings_path)?;

    conn.execute(
        "create table if not exists cat_colors (
             id integer primary key,
             name text not null unique
         )",
        [],
    )?;
    conn.execute(
        "create table if not exists cats (
             id integer primary key,
             name text not null,
             color_id integer not null references cat_colors(id)
         )",
        [],
    )?;

    Ok(())
}

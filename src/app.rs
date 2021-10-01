use eframe::{egui, epi};

use std::fs::create_dir_all;
use std::path::{Path, PathBuf};

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

fn create_config_dir() -> Result<PathBuf> {
    // On windows, dirs's ProjectDirs::from("org", "username", "appname") creates the
    // path "username/appname". See https://github.com/dirs-dev/directories-rs/blob/main/src/win.rs#L94.
    // Does anyone actually like the extra layer of path?
    // In my experience, all it does is make it harder to find the folder corresponding
    // to an app. So I'm omitting the second argument.
    let proj_dirs =
        ProjectDirs::from("", "", "spcplay-rs").context("failed to locate config file dir")?;

    let config_dir: &Path = proj_dirs.config_dir();
    create_dir_all(config_dir).context("creating config file dir")?;

    Ok(config_dir.to_owned())
}

// TODO call in response to menu item
fn touch_sql(conn: &Connection) -> Result<()> {
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

fn open_settings() -> Result<Connection> {
    let config_dir = create_config_dir()?;
    let settings_path = config_dir.join(SETTINGS_NAME);

    let conn = Connection::open(&settings_path)?;

    Ok(conn)
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    // Example stuff:
    label: String,

    // this how you opt-out of serialization of a member
    #[cfg_attr(feature = "persistence", serde(skip))]
    value: f32,

    #[cfg_attr(feature = "persistence", serde(skip))]
    conn: Connection,

    error_dialog: Option<String>,
}

impl TemplateApp {
    pub fn new() -> Result<Self> {
        let conn = open_settings()?;
        Ok(Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            value: 2.7,
            conn,
            error_dialog: None,
        })
    }
}

impl epi::App for TemplateApp {
    fn name(&self) -> &str {
        "egui template"
    }

    /// Called once before the first frame.
    fn setup(
        &mut self,
        _ctx: &egui::CtxRef,
        _frame: &mut epi::Frame<'_>,
        _storage: Option<&dyn epi::Storage>,
    ) {
        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        #[cfg(feature = "persistence")]
        if let Some(storage) = _storage {
            *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
        }
    }

    /// Called by the frame work to save state before shutdown.
    /// Note that you must enable the `persistence` feature for this to work.
    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
        // Examples of how to create different panels and windows.
        // Pick whichever suits you.
        // Tip: a good default choice is to just keep the `CentralPanel`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                egui::menu::menu(ui, "File", |ui| {
                    if ui.button("Quit").clicked() {
                        frame.quit();
                    }
                    if ui.button("Run SQL").clicked() {
                        if let Err(err) = touch_sql(&self.conn) {
                            self.error_dialog = Some(err.to_string());
                        }
                    }
                });
            });
        });

        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            ui.heading("Side Panel");

            ui.horizontal(|ui| {
                ui.label("Write something: ");
                ui.text_edit_singleline(&mut self.label);
            });

            ui.add(egui::Slider::new(&mut self.value, 0.0..=10.0).text("value"));
            if ui.button("Increment").clicked() {
                self.value += 1.0;
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                ui.add(
                    egui::Hyperlink::new("https://github.com/emilk/egui/").text("powered by egui"),
                );
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's

            ui.heading("egui template");
            ui.hyperlink("https://github.com/emilk/egui_template");
            ui.add(egui::github_link_file!(
                "https://github.com/emilk/egui_template/blob/master/",
                "Source code."
            ));
            egui::warn_if_debug_build(ui);
        });

        if let Some(ref msg) = &self.error_dialog {
            // sigh... can't use a single variable for both
            let mut open = true;
            let mut button_pressed = false;

            // open remains borrowed until show() returns.
            egui::Window::new("Error").open(&mut open).show(ctx, |ui| {
                ui.label(msg);
                if ui.button("OK").clicked() {
                    button_pressed = false;
                }
            });

            if !open || button_pressed {
                self.error_dialog = None;
            }
        }
    }
}

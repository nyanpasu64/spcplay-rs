use eframe::{egui, epi};

use std::fs::create_dir_all;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use rusqlite::Connection;

use crate::spcplay::{AudioHandle, SpcPlayer};

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

fn open_settings() -> Result<Connection> {
    let config_dir = create_config_dir()?;
    let settings_path = config_dir.join(SETTINGS_NAME);

    let conn = Connection::open(&settings_path)?;

    Ok(conn)
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

pub struct SpcPlayApp {
    // Settings database connection
    settings: Option<Connection>,

    error_dialog: Option<String>,

    audio: Option<AudioHandle>,
    spc_info: String,
}

impl SpcPlayApp {
    pub fn new() -> Self {
        let settings = open_settings();
        let (settings, error_dialog) = match settings {
            Ok(settings) => (Some(settings), None),
            Err(err) => (None, Some(err.to_string())),
        };

        Self {
            settings,
            error_dialog,
            audio: None,
            spc_info: "".to_owned(),
        }
    }
}

impl epi::App for SpcPlayApp {
    fn name(&self) -> &str {
        "egui template"
    }

    /// Called once before the first frame.
    fn setup(
        &mut self,
        ctx: &egui::CtxRef,
        _frame: &mut epi::Frame<'_>,
        _storage: Option<&dyn epi::Storage>,
    ) {
        // Set fonts.
        {
            static PROPORTIONAL: &str = "B612";
            static MONOSPACE: &str = "Inconsolata";

            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                PROPORTIONAL.to_owned(),
                std::borrow::Cow::Borrowed(include_bytes!("../fonts/B612-Regular.ttf")),
            );
            fonts.font_data.insert(
                MONOSPACE.to_owned(),
                std::borrow::Cow::Borrowed(include_bytes!("../fonts/Inconsolata-Regular.ttf")),
            );
            fonts.fonts_for_family.insert(
                egui::FontFamily::Proportional,
                vec![PROPORTIONAL.to_owned()],
            );
            fonts
                .fonts_for_family
                .insert(egui::FontFamily::Monospace, vec![MONOSPACE.to_owned()]);

            ctx.set_fonts(fonts);
        }

        // Set color theme.
        {
            let mut dark = egui::Visuals::dark();
            let w = &mut dark.widgets;

            let white = egui::Color32::from_gray(0xee);
            for x in [
                &mut w.noninteractive,
                &mut w.inactive,
                &mut w.hovered,
                &mut w.active,
                &mut w.open,
            ] {
                x.fg_stroke.color = white;
            }

            // Setting dark.override_text_color doesn't work.

            ctx.set_visuals(dark);
        }
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
                    if ui.button("Open…").clicked() {
                        if let Err(err) = self.on_open_pressed() {
                            self.error_dialog = Some(err.to_string());
                        }
                    }

                    if let Some(ref settings) = &self.settings {
                        if ui.button("Run SQL").clicked() {
                            if let Err(err) = touch_sql(settings) {
                                self.error_dialog = Some(err.to_string());
                            }
                        }
                    }
                    if ui.button("Quit").clicked() {
                        frame.quit();
                    }
                });
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
                    button_pressed = true;
                }
            });

            if !open || button_pressed {
                self.error_dialog = None;
            }
        }
    }
}

impl SpcPlayApp {
    fn on_open_pressed(&mut self) -> Result<()> {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            let player = SpcPlayer::new(&path)?;

            self.spc_info = player.get_spc_info();

            let audio = AudioHandle::new(player)?;
            self.audio = Some(audio);

            self.audio.as_ref().unwrap().play()?;
        }

        Ok(())
    }
}

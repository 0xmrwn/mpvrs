use tauri::{
  plugin::{Builder, TauriPlugin},
  Manager, Runtime,
};

pub use models::*;

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

mod commands;
mod error;
mod models;

pub use error::{Error, Result};

#[cfg(desktop)]
use desktop::Playa;
#[cfg(mobile)]
use mobile::Playa;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the playa APIs.
pub trait PlayaExt<R: Runtime> {
  fn playa(&self) -> &Playa<R>;
}

impl<R: Runtime, T: Manager<R>> crate::PlayaExt<R> for T {
  fn playa(&self) -> &Playa<R> {
    self.state::<Playa<R>>().inner()
  }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
  Builder::new("playa")
    .invoke_handler(tauri::generate_handler![commands::ping])
    .setup(|app, api| {
      #[cfg(mobile)]
      let playa = mobile::init(app, api)?;
      #[cfg(desktop)]
      let playa = desktop::init(app, api)?;
      app.manage(playa);
      Ok(())
    })
    .build()
}

//! Tuning data: a type's numbers in a RON file, loaded as an asset.
//!
//! Each tuned type (for example `RunnerStats`) has one `.ron` file next to
//! the module that owns it, embedded in the binary with
//! `embedded_tuning!`. [`TuningPlugin`] parses the embedded bytes into
//! [`Tuned<T>`] while the app builds, so the values exist at `Startup` on
//! every platform. It also loads the same file through the asset server:
//! in dev builds (`native` feature) `embedded_watcher` reloads the file
//! when it is saved, and the new values replace [`Tuned<T>`].
//!
//! Code reads [`Tuned<T>`] when it needs a value, so a reload changes what
//! is spawned next, not what already exists.

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, AssetPath, LoadContext};
use bevy::prelude::*;
use serde::de::DeserializeOwned;
use std::fmt;
use std::marker::PhantomData;
use std::path::PathBuf;

/// A type whose values come from a tuning file.
pub trait TuningData: Asset + Clone + DeserializeOwned {
    /// Checks what the type alone cannot, for example that health is
    /// greater than 0. The error says which field is wrong.
    fn validate(&self) -> Result<(), String>;
}

/// Checks that `value` is greater than 0 (NaN is not). For
/// [`TuningData::validate`].
pub fn require_positive(field: &str, value: f32) -> Result<(), String> {
    if value > 0.0 {
        Ok(())
    } else {
        Err(format!("`{field}` must be greater than 0, not {value}"))
    }
}

/// Parses a tuning file and validates its values.
pub fn parse<T: TuningData>(bytes: &[u8]) -> Result<T, TuningError> {
    let values: T = ron::de::from_bytes(bytes)?;
    values.validate().map_err(TuningError::Invalid)?;
    Ok(values)
}

/// Why a tuning file could not be used.
#[derive(Debug)]
pub enum TuningError {
    /// The file could not be read.
    Io(std::io::Error),
    /// The file is not valid RON, or does not match the type: for example
    /// a mistyped field name. The message starts with the line and column.
    Parse(ron::error::SpannedError),
    /// A value is out of range.
    Invalid(String),
}

impl fmt::Display for TuningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "could not read the file: {err}"),
            Self::Parse(err) => {
                let at = &err.span.start;
                write!(f, "line {}, column {}: {}", at.line, at.col, err.code)
            }
            Self::Invalid(reason) => write!(f, "invalid value: {reason}"),
        }
    }
}

impl std::error::Error for TuningError {}

impl From<std::io::Error> for TuningError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<ron::error::SpannedError> for TuningError {
    fn from(err: ron::error::SpannedError) -> Self {
        Self::Parse(err)
    }
}

/// Loads a `.ron` file as `T` and validates it.
///
/// Load with a typed handle (`asset_server.load::<T>(path)`): the asset
/// server picks a loader by asset type first, so every `RonLoader<T>` can
/// share the `ron` extension.
#[derive(TypePath)]
pub struct RonLoader<T>(PhantomData<fn() -> T>);

impl<T> Default for RonLoader<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: TuningData> AssetLoader for RonLoader<T> {
    type Asset = T;
    type Settings = ();
    type Error = TuningError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        parse(&bytes)
    }

    fn extensions(&self) -> &[&str] {
        &["ron"]
    }
}

/// The values of `T` that the game uses. A reload of the file replaces
/// them; components that were made from the old values keep them.
#[derive(Resource)]
pub struct Tuned<T: TuningData>(pub T);

/// The tuning file of `T`. Its strong handle keeps the asset loaded, so
/// the asset server reloads it when the file changes.
#[derive(Resource)]
pub struct TuningFile<T: TuningData> {
    pub path: AssetPath<'static>,
    pub handle: Handle<T>,
}

/// Inserts [`Tuned<T>`] from an embedded tuning file, and replaces it when
/// the file reloads.
///
/// Add it with `embedded_tuning!`, which also embeds the file. If the
/// embedded file is invalid, `build` panics with the path and the error;
/// the tests parse every shipped file. If a reloaded file is invalid, the
/// asset server logs the error and [`Tuned<T>`] keeps its values.
pub struct TuningPlugin<T: TuningData> {
    path: AssetPath<'static>,
    bytes: &'static [u8],
    marker: PhantomData<fn() -> T>,
}

impl<T: TuningData> TuningPlugin<T> {
    /// `path` is the file's path in the `embedded` asset source, as
    /// `embedded_path!` gives it, and `bytes` are its contents.
    pub fn new(path: PathBuf, bytes: &'static [u8]) -> Self {
        Self {
            path: AssetPath::from_path_buf(path).with_source("embedded"),
            bytes,
            marker: PhantomData,
        }
    }
}

impl<T: TuningData> Plugin for TuningPlugin<T> {
    fn build(&self, app: &mut App) {
        let values = match parse::<T>(self.bytes) {
            Ok(values) => values,
            Err(err) => panic!("{}: {err}", self.path),
        };
        app.init_asset::<T>()
            .register_asset_loader(RonLoader::<T>::default())
            .insert_resource(Tuned(values))
            .add_systems(Update, apply_reload::<T>);
        let server = app.world().resource::<AssetServer>();
        let handle: Handle<T> = server.load(self.path.clone());
        app.insert_resource(TuningFile {
            path: self.path.clone(),
            handle,
        });
    }
}

/// Replaces [`Tuned<T>`] with the values of a reloaded tuning file.
fn apply_reload<T: TuningData>(
    mut events: MessageReader<AssetEvent<T>>,
    assets: Res<Assets<T>>,
    file: Res<TuningFile<T>>,
    mut tuned: ResMut<Tuned<T>>,
) {
    for event in events.read() {
        if !event.is_modified(&file.handle) {
            continue;
        }
        let Some(values) = assets.get(&file.handle) else {
            continue;
        };
        tuned.0 = values.clone();
        info!("Reloaded tuning data from {}", file.path);
    }
}

/// Embeds a tuning file and adds a [`TuningPlugin`] that reads it.
///
/// `$file` is relative to the calling file, as in `embedded_asset!`. Its
/// asset path is `embedded://bevy_td_sandbox/<module dir>/<file>`.
///
/// ```ignore
/// // In src/enemy/runner/mod.rs: embeds src/enemy/runner/stats.ron.
/// embedded_tuning!(app, RunnerStats, "stats.ron");
/// ```
macro_rules! embedded_tuning {
    ($app:expr, $ty:ty, $file:literal) => {{
        ::bevy::asset::embedded_asset!($app, $file);
        $app.add_plugins($crate::tuning::TuningPlugin::<$ty>::new(
            ::bevy::asset::embedded_path!($file),
            ::core::include_bytes!($file),
        ));
    }};
}
pub(crate) use embedded_tuning;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{load_tuning, test_app_with_assets, update_until};
    use bevy::asset::LoadState;
    use bevy::asset::io::embedded::EmbeddedAssetRegistry;
    use serde::Deserialize;
    use std::path::Path;

    #[derive(Asset, TypePath, Deserialize, Clone, Debug, PartialEq)]
    #[serde(deny_unknown_fields)]
    struct TestStats {
        health: f32,
        loot: u32,
    }

    impl TuningData for TestStats {
        fn validate(&self) -> Result<(), String> {
            require_positive("health", self.health)
        }
    }

    const PATH: &str = "bevy_td_sandbox/tuning/test.ron";
    const FILE: &[u8] = b"(health: 10.0, loot: 5)";
    const FILE_VALUES: TestStats = TestStats {
        health: 10.0,
        loot: 5,
    };

    /// The error is on line 3 in both files.
    const SYNTAX_ERROR: &str = "(
        health: 10.0,
        loot: 5;
    )";
    const UNKNOWN_FIELD: &str = "(
        health: 10.0,
        lot: 5,
    )";

    /// An app with `TestStats` embedded at `PATH`, as `embedded_tuning!`
    /// would do it.
    fn tuning_app() -> App {
        let mut app = test_app_with_assets();
        app.world()
            .resource::<EmbeddedAssetRegistry>()
            .insert_asset(PathBuf::new(), Path::new(PATH), FILE);
        app.add_plugins(TuningPlugin::<TestStats>::new(PathBuf::from(PATH), FILE));
        app
    }

    fn tuned(app: &App) -> &TestStats {
        &app.world().resource::<Tuned<TestStats>>().0
    }

    #[test]
    fn parse_reads_the_values() {
        assert_eq!(parse::<TestStats>(FILE).unwrap(), FILE_VALUES);
    }

    #[test]
    fn syntax_error_names_the_line() {
        let err = parse::<TestStats>(SYNTAX_ERROR.as_bytes()).unwrap_err();
        assert!(matches!(err, TuningError::Parse(_)), "{err}");
        assert!(err.to_string().starts_with("line 3, column "), "{err}");
    }

    #[test]
    fn unknown_field_names_the_line() {
        let err = parse::<TestStats>(UNKNOWN_FIELD.as_bytes()).unwrap_err();
        assert!(matches!(err, TuningError::Parse(_)), "{err}");
        assert!(err.to_string().starts_with("line 3, column "), "{err}");
        assert!(err.to_string().contains("lot"), "{err}");
    }

    #[test]
    fn value_out_of_range_is_an_error() {
        let file = b"(health: 0.0, loot: 5)";
        let err = parse::<TestStats>(file).unwrap_err();
        assert!(matches!(err, TuningError::Invalid(_)), "{err}");
        assert!(err.to_string().contains("`health`"), "{err}");
    }

    #[test]
    fn require_positive_rejects_zero_negative_and_nan() {
        assert!(require_positive("x", 0.5).is_ok());
        assert!(require_positive("x", 0.0).is_err());
        assert!(require_positive("x", -1.0).is_err());
        assert!(require_positive("x", f32::NAN).is_err());
    }

    #[test]
    fn tuned_exists_once_the_plugin_is_built() {
        let app = tuning_app();
        assert_eq!(*tuned(&app), FILE_VALUES);
    }

    #[test]
    fn file_loads_through_the_asset_server() {
        let mut app = tuning_app();
        load_tuning::<TestStats>(&mut app);
        let handle = &app.world().resource::<TuningFile<TestStats>>().handle;
        let assets = app.world().resource::<Assets<TestStats>>();
        assert_eq!(assets.get(handle), Some(&FILE_VALUES));
    }

    #[test]
    fn modified_asset_replaces_tuned() {
        let mut app = tuning_app();
        load_tuning::<TestStats>(&mut app);
        let file = app.world().resource::<TuningFile<TestStats>>();
        let handle = file.handle.clone();
        let changed = TestStats {
            health: 20.0,
            loot: 7,
        };

        app.world_mut()
            .resource_mut::<Assets<TestStats>>()
            .insert(&handle, changed.clone())
            .unwrap();
        app.update(); // Sends AssetEvent::Modified.
        app.update(); // TuningPlugin reads it.

        assert_eq!(*tuned(&app), changed);
    }

    #[test]
    fn failed_reload_names_the_file_and_line_and_keeps_tuned() {
        for broken in [SYNTAX_ERROR, UNKNOWN_FIELD] {
            let mut app = tuning_app();
            load_tuning::<TestStats>(&mut app);

            // What `embedded_watcher` does when the file is saved.
            app.world()
                .resource::<EmbeddedAssetRegistry>()
                .insert_asset(PathBuf::new(), Path::new(PATH), broken.as_bytes());
            let file = app.world().resource::<TuningFile<TestStats>>();
            let server = app.world().resource::<AssetServer>();
            server.reload(file.path.clone());

            let mut error = String::new();
            update_until(&mut app, |world| {
                let handle = &world.resource::<TuningFile<TestStats>>().handle;
                match world.resource::<AssetServer>().load_state(handle) {
                    LoadState::Failed(err) => {
                        error = err.to_string();
                        true
                    }
                    _ => false,
                }
            });
            app.update();

            assert!(error.contains(PATH), "{error}");
            assert!(error.contains("line 3, column "), "{error}");
            assert_eq!(*tuned(&app), FILE_VALUES, "{error}");
        }
    }
}

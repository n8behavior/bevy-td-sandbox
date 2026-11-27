//! The dev key table: one row for each dev feature.
//!
//! Dev keys are F-keys: `F1`–`F10` and `F12` (`F11` is fullscreen). No dev
//! key is a gameplay key, which `no_dev_key_is_a_gameplay_key` enforces.

use bevy::prelude::*;

/// One dev feature's key, shown in the `F1` key list.
#[derive(Debug, Clone, Copy)]
pub struct DevKey {
    pub key: KeyCode,
    pub name: &'static str,
    /// One line on what the key does.
    pub description: &'static str,
}

/// Shows or hides the list of dev keys.
pub const KEY_LIST: KeyCode = KeyCode::F1;

/// Every dev key, in the order the key list shows them.
pub const DEV_KEYS: &[DevKey] = &[DevKey {
    key: KEY_LIST,
    name: "Dev keys",
    description: "Show or hide this list",
}];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::path::Path;

    /// Adds every name that follows `KeyCode::` in the `.rs` files under
    /// `dir` to `names`, skipping the `skip` directory.
    fn collect_key_names(dir: &Path, skip: &Path, names: &mut HashSet<String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path == skip {
                continue;
            }
            if path.is_dir() {
                collect_key_names(&path, skip, names);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let source = std::fs::read_to_string(&path).unwrap();
                for rest in source.split("KeyCode::").skip(1) {
                    let name: String = rest
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                        .collect();
                    if !name.is_empty() {
                        names.insert(name);
                    }
                }
            }
        }
    }

    #[test]
    fn no_dev_key_is_a_gameplay_key() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut gameplay_keys = HashSet::new();
        collect_key_names(&src, &src.join("dev"), &mut gameplay_keys);

        // Keys the game is known to use, so a broken scan cannot pass.
        for known in ["KeyR", "Enter", "F11"] {
            assert!(
                gameplay_keys.contains(known),
                "the scan of src/ did not find KeyCode::{known}"
            );
        }

        let mut dev_keys = HashSet::new();
        for row in DEV_KEYS {
            let key = format!("{:?}", row.key);
            assert!(
                !gameplay_keys.contains(&key),
                "dev key {key} ({}) is also a gameplay key",
                row.name
            );
            assert!(dev_keys.insert(key.clone()), "{key} is in DEV_KEYS twice");
        }
    }

    #[test]
    fn dev_keys_are_f_keys() {
        const ALLOWED: [KeyCode; 11] = [
            KeyCode::F1,
            KeyCode::F2,
            KeyCode::F3,
            KeyCode::F4,
            KeyCode::F5,
            KeyCode::F6,
            KeyCode::F7,
            KeyCode::F8,
            KeyCode::F9,
            KeyCode::F10,
            KeyCode::F12,
        ];
        for row in DEV_KEYS {
            assert!(
                ALLOWED.contains(&row.key),
                "dev key {:?} ({}) is not F1-F10 or F12",
                row.key,
                row.name
            );
        }
    }
}

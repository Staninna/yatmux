use super::*;

impl Config {
    pub(super) fn deep_merge(dst: &mut toml::Value, src: toml::Value) {
        match (dst, src) {
            (toml::Value::Table(dst_table), toml::Value::Table(src_table)) => {
                for (k, v) in src_table {
                    match dst_table.get_mut(&k) {
                        Some(existing) => Self::deep_merge(existing, v),
                        None => {
                            dst_table.insert(k, v);
                        }
                    }
                }
            }
            (dst_slot, src_value) => {
                *dst_slot = src_value;
            }
        }
    }

    pub(super) fn apply_theme_overrides(merged: &mut toml::Value, theme: &toml::Value) {
        let Some(theme_table) = theme.as_table() else {
            return;
        };

        // Precedence for native UI colors: theme > [ui] > [colors]
        for key in ["ui", "colors"] {
            let Some(theme_part) = theme_table.get(key) else {
                continue;
            };

            let mut slot = merged
                .get(key)
                .cloned()
                .unwrap_or_else(|| toml::Value::Table(toml::map::Map::new()));

            Self::deep_merge(&mut slot, theme_part.clone());

            if let Some(dst_table) = merged.as_table_mut() {
                dst_table.insert(key.to_string(), slot);
            }
        }
    }
}

#[cfg(test)]
mod merge_tests {
    use super::Config;

    #[test]
    fn theme_overrides_ui_and_colors() {
        let mut merged: toml::Value = r##"
            [colors]
            background = "#000"

            [ui.toast]
            duration_ms = 123
        "##
        .parse()
        .unwrap();

        let theme: toml::Value = r##"
            [colors]
            background = "#111"

            [ui.toast]
            duration_ms = 999
        "##
        .parse()
        .unwrap();

        Config::apply_theme_overrides(&mut merged, &theme);

        let merged_table = merged.as_table().unwrap();
        let colors = merged_table.get("colors").unwrap().as_table().unwrap();
        assert_eq!(colors.get("background").unwrap().as_str().unwrap(), "#111");

        let ui = merged_table.get("ui").unwrap().as_table().unwrap();
        let toast = ui.get("toast").unwrap().as_table().unwrap();
        assert_eq!(toast.get("duration_ms").unwrap().as_integer().unwrap(), 999);
    }
}

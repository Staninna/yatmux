pub(crate) mod hex_color {
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(color: &u32, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("#{:06X}", color))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u32, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;

        // Accept either a string ("#RRGGBB", "0xRRGGBB") or an integer
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ColorValue {
            String(String),
            Int(u32),
        }

        match ColorValue::deserialize(deserializer)? {
            ColorValue::Int(n) => Ok(n),
            ColorValue::String(s) => {
                parse_color(&s).ok_or_else(|| Error::custom("invalid color format"))
            }
        }
    }

    fn parse_color(s: &str) -> Option<u32> {
        let s = s.trim();
        let s = s.trim_start_matches('#').trim_start_matches("0x");

        // #RGB shorthand -> #RRGGBB
        if s.len() == 3 {
            let mut expanded = String::with_capacity(6);
            for ch in s.chars() {
                expanded.push(ch);
                expanded.push(ch);
            }
            return u32::from_str_radix(&expanded, 16).ok();
        }

        u32::from_str_radix(s, 16).ok()
    }
}

pub(crate) mod hex_color_opt {
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(color: &Option<u32>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match color {
            Some(c) => super::hex_color::serialize(c, serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Some(super::hex_color::deserialize(deserializer)?))
    }
}

pub(crate) mod hex_palette_opt {
    use serde::{Deserialize, Deserializer, Serializer};

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ColorValue {
        String(String),
        Int(u32),
    }

    fn parse_color(s: &str) -> Option<u32> {
        let s = s.trim();
        let s = s.trim_start_matches('#').trim_start_matches("0x");

        if s.len() == 3 {
            let mut expanded = String::with_capacity(6);
            for ch in s.chars() {
                expanded.push(ch);
                expanded.push(ch);
            }
            return u32::from_str_radix(&expanded, 16).ok();
        }

        u32::from_str_radix(s, 16).ok()
    }

    pub fn serialize<S>(palette: &Option<[u32; 16]>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match palette {
            None => serializer.serialize_none(),
            Some(p) => {
                let items: Vec<String> = p.iter().map(|c| format!("#{:06X}", c)).collect();
                serializer.serialize_some(&items)
            }
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<[u32; 16]>, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;

        let opt = Option::<Vec<ColorValue>>::deserialize(deserializer)?;
        let Some(values) = opt else {
            return Ok(None);
        };

        if values.len() != 16 {
            return Err(Error::custom("palette must have exactly 16 colors"));
        }

        let mut out = [0u32; 16];
        for (i, v) in values.into_iter().enumerate() {
            out[i] = match v {
                ColorValue::Int(n) => n,
                ColorValue::String(s) => {
                    parse_color(&s).ok_or_else(|| Error::custom("invalid color format"))?
                }
            };
        }

        Ok(Some(out))
    }
}

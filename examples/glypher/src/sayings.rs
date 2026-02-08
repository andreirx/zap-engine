/// Database of sayings/proverbs for the game.
pub struct SayingsDB {
    sayings: Vec<String>,
}

impl SayingsDB {
    /// Parse a JSON array of strings.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let sayings: Vec<String> = serde_json::from_str(json)?;
        Ok(Self { sayings })
    }

    /// Pick a saying by index (caller provides random index).
    pub fn pick(&self, index: usize) -> &str {
        &self.sayings[index % self.sayings.len()]
    }

    /// Number of sayings in the database.
    pub fn len(&self) -> usize {
        self.sayings.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sayings() {
        let json = r#"["hello world", "foo bar"]"#;
        let db = SayingsDB::from_json(json).unwrap();
        assert_eq!(db.len(), 2);
    }

    #[test]
    fn pick_wraps_around() {
        let json = r#"["alpha", "beta", "gamma"]"#;
        let db = SayingsDB::from_json(json).unwrap();
        assert_eq!(db.pick(0), "alpha");
        assert_eq!(db.pick(1), "beta");
        assert_eq!(db.pick(5), "gamma"); // 5 % 3 = 2
    }
}

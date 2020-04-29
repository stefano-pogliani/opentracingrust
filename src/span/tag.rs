use std::collections::HashMap;
use std::collections::hash_map::Iter;


/// Map strings to `TagValue`s.
///
/// This structure is a tailored wrapper around `HashMap`s.
#[derive(Debug, Default)]
pub struct SpanTags(HashMap<String, TagValue>);

impl SpanTags {
    /// Returns a new empty tag map.
    pub fn new() -> SpanTags {
        SpanTags(HashMap::new())
    }
}

impl SpanTags {
    /// Attempt to extract a tag by name.
    pub fn get(&self, tag: &str) -> Option<&TagValue> {
        self.0.get(tag)
    }

    /// Returns an iteratore over all tags.
    pub fn iter(&self) -> Iter<String, TagValue> {
        self.0.iter()
    }

    /// Set a tag to the given value.
    pub fn tag(&mut self, tag: &str, value: TagValue) {
        self.0.insert(String::from(tag), value);
    }
}


/// Enumeration of valid types for tag values.
#[derive(Debug)]
pub enum TagValue {
    Boolean(bool),
    Float(f64),
    Integer(i64),
    String(String),
}

impl From<bool> for TagValue {
    fn from(value: bool) -> TagValue {
        TagValue::Boolean(value)
    }
}

impl From<f64> for TagValue {
    fn from(value: f64) -> TagValue {
        TagValue::Float(value)
    }
}

impl From<i64> for TagValue {
    fn from(value: i64) -> TagValue {
        TagValue::Integer(value)
    }
}

impl<'a> From<&'a str> for TagValue {
    fn from(value: &'a str) -> TagValue {
        TagValue::String(String::from(value))
    }
}

impl From<String> for TagValue {
    fn from(value: String) -> TagValue {
        TagValue::String(value)
    }
}


#[cfg(test)]
mod tests {
    use super::SpanTags;
    use super::TagValue;

    #[test]
    fn get_missing_tag() {
        let tags = SpanTags::new();
        match tags.get("key") {
            Some(_) => panic!("Expected no tag"),
            None => {}
        }
    }

    #[test]
    fn iterate_over_tags() {
        let mut tags = SpanTags::new();
        tags.tag("key", TagValue::Integer(42));
        for (key, value) in tags.iter() {
            assert_eq!(key, "key");
            match value {
                &TagValue::Integer(i) => assert_eq!(i, 42),
                _ => panic!("Invalid value type")
            }
        }
    }

    #[test]
    fn set_tag() {
        let mut tags = SpanTags::new();
        tags.tag("key", TagValue::Integer(42));
        match tags.get("key") {
            Some(&TagValue::Integer(i)) => assert_eq!(i, 42),
            Some(_) => panic!("Invalid value type"),
            None => panic!("Tag not found")
        }
    }
}

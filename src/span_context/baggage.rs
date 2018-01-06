/// TODO
#[derive(Debug, Clone, PartialEq)]
pub struct BaggageItem {
    key: String,
    value: String,
}

impl BaggageItem {
    /// TODO
    pub fn new<K, V>(key: K, value: V) -> BaggageItem
        where K: Into<String>,
              V: Into<String>
    {
        BaggageItem {
            key: key.into(),
            value: value.into()
        }
    }

    /// TODO
    pub fn key(&self) -> &str {
        &self.key
    }

    /// TODO
    pub fn value(&self) -> &str {
        &self.value
    }
}


#[cfg(test)]
mod tests {
    use super::BaggageItem;

    #[test]
    fn from_str() {
        let item = BaggageItem::new("key", "value");
        assert_eq!(item.key(), "key");
        assert_eq!(item.value(), "value");
    }

    #[test]
    fn from_string() {
        let item = BaggageItem::new(String::from("key"), String::from("value"));
        assert_eq!(item.key(), "key");
        assert_eq!(item.value(), "value");
    }
}

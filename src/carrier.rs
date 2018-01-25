use std::collections::BTreeMap;
use std::collections::HashMap;
use std::io;


/// `SpanContext` extraction format and source.
///
/// Each supported extraction format also carries an object trait to
/// the data carrier the `SpanContext` should be extracted from.
///
/// # Examples
///
/// ```
/// extern crate opentracingrust;
///
/// use std::collections::HashMap;
/// use opentracingrust::ExtractFormat;
///
///
/// fn main() {
///     let mut headers: HashMap<String, String> = HashMap::new();
///     headers.insert(String::from("TraceId"), String::from("123"));
///     headers.insert(String::from("SpanId"), String::from("456"));
///
///     let format = ExtractFormat::HttpHeaders(Box::new(&headers)); 
///     // ... snip ...
/// }
/// ```
pub enum ExtractFormat<'a> {
    Binary(Box<&'a mut self::io::Read>),
    HttpHeaders(Box<&'a MapCarrier>),
    TextMap(Box<&'a MapCarrier>)
}


/// `SpanContext` injection format and destination.
///
/// Each supported injection format also carries an object trait to
/// the data carrier the `SpanContext` should be injected into.
///
/// # Examples
///
/// ```
/// extern crate opentracingrust;
///
/// use std::collections::HashMap;
/// use opentracingrust::InjectFormat;
///
///
/// fn main() {
///     let mut headers: HashMap<String, String> = HashMap::new();
///     let format = InjectFormat::HttpHeaders(Box::new(&mut headers)); 
///     // ... snip ...
/// }
/// ```
pub enum InjectFormat<'a> {
    Binary(Box<&'a mut self::io::Write>),
    HttpHeaders(Box<&'a mut MapCarrier>),
    TextMap(Box<&'a mut MapCarrier>)
}


/// Interface for HTTP header and text map carriers.
///
/// A trait used by `InjectFormat` and `ExtractFormat` to store carriers that
/// support the `HttpHeaders` and the `TextMap` formats.
pub trait MapCarrier {
    /// List all items stored in the carrier as `(key, value)` pairs.
    ///
    /// Intended to be used by the `ExtractFormat`s to extract all the
    /// baggage items from the carrier.
    fn items(&self) -> Vec<(&String, &String)>;

    /// Attempt to fetch an exact key from the carrier.
    fn get(&self, key: &str) -> Option<String>;

    /// Set a key/value pair on the carrier.
    fn set(&mut self, key: &str, value: &str);
}

impl MapCarrier for HashMap<String, String> {
    fn items(&self) -> Vec<(&String, &String)> {
        self.iter().collect()
    }

    fn get(&self, key: &str) -> Option<String> {
        self.get(key).map(|v| v.clone())
    }

    fn set(&mut self, key: &str, value: &str) {
        self.insert(String::from(key), String::from(value));
    }
}

impl MapCarrier for BTreeMap<String, String> {
    fn items(&self) -> Vec<(&String, &String)> {
        self.iter().collect()
    }

    fn get(&self, key: &str) -> Option<String> {
        self.get(key).map(|v| v.clone())
    }

    fn set(&mut self, key: &str, value: &str) {
        self.insert(String::from(key), String::from(value));
    }
}


#[cfg(test)]
mod tests {
    mod tree_map {
        use std::collections::BTreeMap;
        use super::super::MapCarrier;

        #[test]
        fn extract_keys() {
            let mut tree: BTreeMap<String, String> = BTreeMap::new();
            tree.insert(String::from("aa"), String::from("d"));
            assert_eq!(tree.get("aa").unwrap(), "d");
        }

        #[test]
        fn find_keys() {
            let mut tree: BTreeMap<String, String> = BTreeMap::new();
            tree.insert(String::from("aa"), String::from("d"));
            tree.insert(String::from("ab"), String::from("e"));
            tree.insert(String::from("bc"), String::from("f"));

            let mut items = vec![];
            for (key, value) in tree.items() {
                if key.starts_with("a") {
                    items.push((key.clone(), value.clone()));
                }
            }
            items.sort();
            assert_eq!(items, [
                (String::from("aa"), String::from("d")),
                (String::from("ab"), String::from("e"))
            ]);
        }

        #[test]
        fn inject_keys() {
            let mut tree: BTreeMap<String, String> = BTreeMap::new();
            tree.set("a", "d");
            tree.set("b", "e");
            tree.set("c", "f");
            assert_eq!("d", tree.get("a").unwrap());
            assert_eq!("e", tree.get("b").unwrap());
            assert_eq!("f", tree.get("c").unwrap());
        }
    }

    mod hash_map {
        use std::collections::HashMap;
        use super::super::MapCarrier;

        #[test]
        fn extract_keys() {
            let mut map: HashMap<String, String> = HashMap::new();
            map.insert(String::from("aa"), String::from("d"));
            assert_eq!(map.get("aa").unwrap(), "d");
        }

        #[test]
        fn find_keys() {
            let mut map: HashMap<String, String> = HashMap::new();
            map.insert(String::from("aa"), String::from("d"));
            map.insert(String::from("ab"), String::from("e"));
            map.insert(String::from("bc"), String::from("f"));

            let mut items = vec![];
            for (key, value) in map.items() {
                if key.starts_with("a") {
                    items.push((key.clone(), value.clone()));
                }
            }
            items.sort();
            assert_eq!(items, [
                (String::from("aa"), String::from("d")),
                (String::from("ab"), String::from("e"))
            ]);
        }

        #[test]
        fn inject_keys() {
            let mut map: HashMap<String, String> = HashMap::new();
            map.set("a", "d");
            map.set("b", "e");
            map.set("c", "f");
            assert_eq!("d", map.get("a").unwrap());
            assert_eq!("e", map.get("b").unwrap());
            assert_eq!("f", map.get("c").unwrap());
        }
    }
}

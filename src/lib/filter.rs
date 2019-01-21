// This represents a filtering query. This can be done
// in parallel map/reduce style, or directly on a single
// entry to assert it matches.

use super::entry::Entry;
use std::cmp::{Ordering, PartialOrd};
use regex::Regex;

// Perhaps make these json serialisable. Certainly would make parsing
// simpler ...

#[derive(Serialize, Deserialize, Debug)]
pub enum Filter {
    // This is attr - value
    Eq(String, String),
    Sub(String, String),
    Pres(String),
    Or(Vec<Filter>),
    And(Vec<Filter>),
    Not(Box<Filter>),
}

impl Filter {
    // Does this need mut self? Aren't we returning
    // a new copied filter?
    pub fn optimise(&self) -> Self {
        // Apply optimisations to the filter
        // An easy way would be imple partialOrd
        // then do sort on the or/and/not
        // as the general conditions we want
        // to optimise on are in those ...
        //
        // The other big one is folding redundant
        // terms down.
        //
        // If an or/not/and condition has no items, remove it
        //
        // If its the root item?
        self.clone()
    }

    // What other parse types do we need?

    // FIXME: This check should be in ENTRY not here, because it's up to others
    // to interpret filter meaning and application!!!

    // Assert if this filter matches the entry (no index)
    pub fn entry_match_no_index(&self, e: &Entry) -> bool {
        // Go through the filter components and check them in the entry.
        // This is recursive!!!!
        match self {
            Filter::Eq(attr, value) => e.attribute_equality(attr.as_str(), value.as_str()),
            Filter::Sub(attr, subvalue) => e.attribute_substring(attr.as_str(), subvalue.as_str()),
            Filter::Pres(attr) => {
                // Given attr, is is present in the entry?
                e.attribute_pres(attr.as_str())
            }
            Filter::Or(l) => {
                l.iter()
                    .fold(false, |acc, f| {
                        if acc {
                            acc
                        } else {
                            f.entry_match_no_index(e)
                        }
                    })
            }
            Filter::And(l) => {
                l.iter()
                    .fold(true, |acc, f| {
                        if acc {
                            f.entry_match_no_index(e)
                        } else {
                            acc
                        }
                    })
            }
            Filter::Not(f) => {
                !f.entry_match_no_index(e)
            }
        }
    }
}

impl Clone for Filter {
    fn clone(&self) -> Self {
        // I think we only need to match self then new + clone?
        match self {
            Filter::Eq(a, v) => Filter::Eq(a.clone(), v.clone()),
            Filter::Sub(a, v) => Filter::Sub(a.clone(), v.clone()),
            Filter::Pres(a) => Filter::Pres(a.clone()),
            Filter::Or(l) => Filter::Or(l.clone()),
            Filter::And(l) => Filter::And(l.clone()),
            Filter::Not(l) => Filter::Not(l.clone()),
        }
    }
}

impl PartialEq for Filter {
    fn eq(&self, rhs: &Filter) -> bool {
        match (self, rhs) {
            (Filter::Eq(a1, v1), Filter::Eq(a2, v2)) => a1 == a2 && v1 == v2,
            (Filter::Sub(a1, v1), Filter::Sub(a2, v2)) => a1 == a2 && v1 == v2,
            (Filter::Pres(a1), Filter::Pres(a2)) => a1 == a2,
            (Filter::Or(l1), Filter::Or(l2)) => l1 == l2,
            (Filter::And(l1), Filter::And(l2)) => l1 == l2,
            (Filter::Not(l1), Filter::Not(l2)) => l1 == l2,
            (_, _) => false,
        }
    }
}

// remember, this isn't ordering by alphanumeric, this is ordering of
// optimisation preference!
impl PartialOrd for Filter {
    fn partial_cmp(&self, rhs: &Filter) -> Option<Ordering> {
        match (self, rhs) {
            (Filter::Eq(a1, _), Filter::Eq(a2, _)) => {
                // Order attr name, then value
                // Later we may add rules to put certain attrs ahead due
                // to optimisation rules
                a1.partial_cmp(a2)
            }
            (Filter::Sub(a1, _), Filter::Sub(a2, _)) => a1.partial_cmp(a2),
            (Filter::Pres(a1), Filter::Pres(a2)) => a1.partial_cmp(a2),
            (Filter::Eq(_, _), _) => {
                // Always higher prefer Eq over all else, as these will have
                // the best indexes and return smallest candidates.
                Some(Ordering::Less)
            }
            (_, Filter::Eq(_, _)) => Some(Ordering::Greater),
            (Filter::Pres(_), _) => Some(Ordering::Less),
            (_, Filter::Pres(_)) => Some(Ordering::Greater),
            (Filter::Sub(_, _), _) => Some(Ordering::Greater),
            (_, Filter::Sub(_, _)) => Some(Ordering::Less),
            (_, _) => Some(Ordering::Equal),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Filter;
    use entry::Entry;
    use serde_json;
    use std::cmp::{Ordering, PartialOrd};

    #[test]
    fn test_filter_simple() {
        let filt = Filter::Eq(String::from("class"), String::from("user"));
        let j = serde_json::to_string_pretty(&filt);
        println!("{}", j.unwrap());

        let complex_filt = Filter::And(vec![
            Filter::Or(vec![
                Filter::Eq(String::from("userid"), String::from("test_a")),
                Filter::Eq(String::from("userid"), String::from("test_b")),
            ]),
            Filter::Eq(String::from("class"), String::from("user")),
        ]);
        let y = serde_json::to_string_pretty(&complex_filt);
        println!("{}", y.unwrap());
    }

    #[test]
    fn test_filter_optimise() {
        // Given sets of "optimisable" filters, optimise them.
    }

    #[test]
    fn test_filter_eq() {
        let f_t1a = Filter::Pres(String::from("userid"));
        let f_t1b = Filter::Pres(String::from("userid"));
        let f_t1c = Filter::Pres(String::from("zzzz"));

        assert_eq!(f_t1a == f_t1b, true);
        assert_eq!(f_t1a == f_t1c, false);
        assert_eq!(f_t1b == f_t1c, false);

        let f_t2a = Filter::And(vec![f_t1a]);
        let f_t2b = Filter::And(vec![f_t1b]);
        let f_t2c = Filter::And(vec![f_t1c]);
        assert_eq!(f_t2a == f_t2b, true);
        assert_eq!(f_t2a == f_t2c, false);
        assert_eq!(f_t2b == f_t2c, false);

        assert_eq!(f_t2c == Filter::Pres(String::from("test")), false);
    }

    #[test]
    fn test_filter_ord() {
        // Test that we uphold the rules of partialOrd
        // Basic equality
        // Test the two major paths here (str vs list)
        let f_t1a = Filter::Pres(String::from("userid"));
        let f_t1b = Filter::Pres(String::from("userid"));

        assert_eq!(f_t1a.partial_cmp(&f_t1b), Some(Ordering::Equal));
        assert_eq!(f_t1b.partial_cmp(&f_t1a), Some(Ordering::Equal));

        let f_t2a = Filter::And(vec![]);
        let f_t2b = Filter::And(vec![]);
        assert_eq!(f_t2a.partial_cmp(&f_t2b), Some(Ordering::Equal));
        assert_eq!(f_t2b.partial_cmp(&f_t2a), Some(Ordering::Equal));

        // antisymmetry: if a < b then !(a > b), as well as a > b implying !(a < b); and
        let f_t3b = Filter::Eq(String::from("userid"), String::from(""));
        assert_eq!(f_t1a.partial_cmp(&f_t3b), Some(Ordering::Greater));
        assert_eq!(f_t3b.partial_cmp(&f_t1a), Some(Ordering::Less));

        // transitivity: a < b and b < c implies a < c. The same must hold for both == and >.
        let f_t4b = Filter::Sub(String::from("userid"), String::from(""));
        assert_eq!(f_t1a.partial_cmp(&f_t4b), Some(Ordering::Less));
        assert_eq!(f_t3b.partial_cmp(&f_t4b), Some(Ordering::Less));

        assert_eq!(f_t4b.partial_cmp(&f_t1a), Some(Ordering::Greater));
        assert_eq!(f_t4b.partial_cmp(&f_t3b), Some(Ordering::Greater));
    }

    #[test]
    fn test_filter_clone() {
        // Test that cloning filters yields the same result regardless of
        // complexity.
        let f_t1a = Filter::Pres(String::from("userid"));
        let f_t1b = f_t1a.clone();
        let f_t1c = Filter::Pres(String::from("zzzz"));

        assert_eq!(f_t1a == f_t1b, true);
        assert_eq!(f_t1a == f_t1c, false);

        let f_t2a = Filter::And(vec![f_t1a]);
        let f_t2b = f_t2a.clone();
        let f_t2c = Filter::And(vec![f_t1c]);

        assert_eq!(f_t2a == f_t2b, true);
        assert_eq!(f_t2a == f_t2c, false);
    }

    #[test]
    fn test_or_entry_filter() {
        let e: Entry = serde_json::from_str(r#"{
            "attrs": {
                "userid": ["william"],
                "uidNumber": ["1000"]
            }
        }"#).unwrap();

        let f_t1a = Filter::Or(vec![
            Filter::Eq(String::from("userid"), String::from("william")),
            Filter::Eq(String::from("uidNumber"), String::from("1000")),
        ]);
        assert!(f_t1a.entry_match_no_index(&e));

        let f_t2a = Filter::Or(vec![
            Filter::Eq(String::from("userid"), String::from("william")),
            Filter::Eq(String::from("uidNumber"), String::from("1001")),
        ]);
        assert!(f_t2a.entry_match_no_index(&e));

        let f_t3a = Filter::Or(vec![
            Filter::Eq(String::from("userid"), String::from("alice")),
            Filter::Eq(String::from("uidNumber"), String::from("1000")),
        ]);
        assert!(f_t3a.entry_match_no_index(&e));

        let f_t4a = Filter::Or(vec![
            Filter::Eq(String::from("userid"), String::from("alice")),
            Filter::Eq(String::from("uidNumber"), String::from("1001")),
        ]);
        assert!(!f_t4a.entry_match_no_index(&e));
    }

    #[test]
    fn test_and_entry_filter() {
        let e: Entry = serde_json::from_str(r#"{
            "attrs": {
                "userid": ["william"],
                "uidNumber": ["1000"]
            }
        }"#).unwrap();

        let f_t1a = Filter::And(vec![
            Filter::Eq(String::from("userid"), String::from("william")),
            Filter::Eq(String::from("uidNumber"), String::from("1000")),
        ]);
        assert!(f_t1a.entry_match_no_index(&e));

        let f_t2a = Filter::And(vec![
            Filter::Eq(String::from("userid"), String::from("william")),
            Filter::Eq(String::from("uidNumber"), String::from("1001")),
        ]);
        assert!(!f_t2a.entry_match_no_index(&e));

        let f_t3a = Filter::And(vec![
            Filter::Eq(String::from("userid"), String::from("alice")),
            Filter::Eq(String::from("uidNumber"), String::from("1000")),
        ]);
        assert!(!f_t3a.entry_match_no_index(&e));

        let f_t4a = Filter::And(vec![
            Filter::Eq(String::from("userid"), String::from("alice")),
            Filter::Eq(String::from("uidNumber"), String::from("1001")),
        ]);
        assert!(!f_t4a.entry_match_no_index(&e));
    }

    #[test]
    fn test_not_entry_filter() {
        let e1: Entry = serde_json::from_str(r#"{
            "attrs": {
                "userid": ["william"],
                "uidNumber": ["1000"]
            }
        }"#).unwrap();
        assert!(false);
    }

    #[test]
    fn test_nested_entry_filter() {
        let e1: Entry = serde_json::from_str(r#"{
            "attrs": {
                "class": ["person"],
                "uidNumber": ["1000"]
            }
        }"#).unwrap();

        let e2: Entry = serde_json::from_str(r#"{
            "attrs": {
                "class": ["person"],
                "uidNumber": ["1001"]
            }
        }"#).unwrap();

        let e3: Entry = serde_json::from_str(r#"{
            "attrs": {
                "class": ["person"],
                "uidNumber": ["1002"]
            }
        }"#).unwrap();

        let e4: Entry = serde_json::from_str(r#"{
            "attrs": {
                "class": ["group"],
                "uidNumber": ["1000"]
            }
        }"#).unwrap();

        let f_t1a = Filter::And(vec![
            Filter::Eq(String::from("class"), String::from("person")),
            Filter::Or(vec![
                Filter::Eq(String::from("uidNumber"), String::from("1001")),
                Filter::Eq(String::from("uidNumber"), String::from("1000")),
            ]),
        ]);

        assert!(f_t1a.entry_match_no_index(&e1));
        assert!(f_t1a.entry_match_no_index(&e2));
        assert!(!f_t1a.entry_match_no_index(&e3));
        assert!(!f_t1a.entry_match_no_index(&e4));
    }
}
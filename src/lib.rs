use std::cmp;
use std::fs;
use std::io;
use std::collections::{HashMap,HashSet};
use bincode;
use serde::{Serialize,Deserialize};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_tests() {
        serialize("aba", false);
        serialize("jfkds.}laN= -;a|ba", false);
        serialize("asab", false);
    }

    fn serialize(contents: &str, check_reconstruct: bool) {
        let trie = SuffixTrie::new(contents);
        let encoded: Vec<u8> = bincode::serialize(&trie).unwrap();
        let decoded: SuffixTrie = bincode::deserialize(&encoded[..]).unwrap();

        println!("{:#?}", trie);
        println!("{:#?}", decoded);
        if check_reconstruct {
            // The debug formatting is not definitive e.g. keys will
            // appear in a different order, but the structure is the same
            // So we can only use this check in certain situations e.g. "aba"
            assert_eq!(format!("{:#?}", trie),
                       format!("{:#?}", decoded));
        }
    }

    #[test]
    fn test_size() {
        let trie = SuffixTrie::new("aba");
        println!("Result is {:#?}", trie);
        assert_eq!(trie.len(), 6);
    }

    #[test]
    fn test_leaves() {
        let trie = SuffixTrie::new("aba");
        println!("Result is {:#?}", trie);

        let expected: HashSet<usize> = (0..3).collect();
        // Gather together all leaf children from the SuffixTrie
        let mut actual: HashSet<usize> = HashSet::new();
        for node in trie.node_storage.iter() {
            for leaf_child in node.leaf_children.iter() {
                // Insert node to list, and assert that it wasn't already present
                assert!(actual.insert(*leaf_child));
            }
        }
        // Check for equality
        assert!(actual.is_superset(&expected));
        assert!(expected.is_superset(&actual));
    }

    #[test]
    fn find_matches() {
        let trie = SuffixTrie::new("aba");
        println!("Result is {:#?}", trie);

        let matches = trie.find_exact("a");
        assert_eq!(matches, vec![0, 2]);

        let trie = SuffixTrie::new("bananaBal");
        println!("Result is {:#?}", trie);

        let matches = trie.find_exact("an");
        assert_eq!(matches, vec![1, 3]);

        let matches = trie.find_exact("ab");
        assert_eq!(matches, vec![]);
    }

    #[test]
    fn find_matches_0_edit() {
        let trie = SuffixTrie::new("aba");
        println!("Result is {:#?}", trie);

        let matches = trie.find_edit_distance("a", 0);
        assert_eq!(matches, vec![0, 2]);

        let trie = SuffixTrie::new("bananaBal");
        println!("Result is {:#?}", trie);

        let matches = trie.find_edit_distance("an", 0);
        assert_eq!(matches, vec![1, 3]);

        let matches = trie.find_edit_distance("ab", 0);
        assert_eq!(matches, vec![]);
    }

    #[test]
    fn find_matches_mismatch() {
        let trie = SuffixTrie::new("barbazbanboo");
        println!("Result is {:#?}", trie);

        let matches = trie.find_edit_distance("bar", 1);
        assert_eq!(matches, vec![0, 3, 6]);

        let matches = trie.find_edit_distance("bar", 2);
        assert_eq!(matches, vec![0, 3, 6, 9]);
    }

    #[test]
    fn find_matches_insert_delete() {
        let trie = SuffixTrie::new("abcXd");
        println!("Result is {:#?}", trie);

        // Delete from text
        let matches = trie.find_edit_distance("abcd", 1);
        assert_eq!(matches, vec![0]);

        // Delete from pattern
        let matches = trie.find_edit_distance("aXbc", 1);
        assert_eq!(matches, vec![0]);
    }

    #[test]
    fn find_partial_matches_ignore() {
        let trie = SuffixTrie::new("He wracked wrack'd wrack'ed");
        println!("Result is {:#?}", trie);

        let mut ignored = HashMap::new();
        ignored.insert('e', true);
        ignored.insert('\'', true);
        let matches = trie.find_edit_distance_ignore("wrackd", 0, ignored.clone());
        assert_eq!(matches, vec![3, 11, 19]);
        let matches = trie.find_edit_distance_ignore("wrack'de", 0, ignored.clone());
        assert_eq!(matches, vec![3, 11, 19]);
    }

    fn find_single_wildcard() {
        let trie = SuffixTrie::new("oh this and that");
        println!("Result is {:#?}", trie);
        let matches = trie.find_edit_distance_ignore("th??", 0, HashMap::new());
        assert_eq!(matches, vec![3, 12]);
    }

    #[test]
    fn construct_trie_from_file() {
        let trie = SuffixTrie::from_file("resources/tests/small.txt");
    }
}

const SINGLE_WILDCARD: char = '?';

#[derive(Clone,Copy,Debug)]
struct Match {
    node_index: usize,
    errors: usize,
}

#[derive(Debug,Serialize,Deserialize)]
struct SuffixTrie {
    // Place to store entire string - keeps ownership simple
    str_storage: String,
    // Place to store all the nodes
    node_storage: Vec<SubTrie>,
}

#[derive(Debug,Serialize,Deserialize)]
struct SubTrie {
    // Index of this node in the overall array
    node_index: usize,
    // List of children node indices, indexed by the character labelling the edge
    // from the parent to the child
    children: HashMap<char, usize>,
    // List of indices at which this suffix is present
    leaf_children: Vec<usize>,
}

impl Match {
    fn new(node_index: usize, errors: usize) -> Self {
        Match {
            node_index,
            errors,
        }
    }
}

impl SuffixTrie {
    /// New suffix trie containing suffixes of a single string
    fn new(string: &str) -> Self {
        let mut suffix_trie = SuffixTrie::empty();
        suffix_trie.add_string_suffixes(string);
        suffix_trie
    }

    /// New empty suffix trie
    fn empty() -> Self {
        let root_node = SubTrie::empty(0);
        let mut suffix_trie = SuffixTrie {
            str_storage: String::from(""),
            node_storage: vec![root_node],
        };
        suffix_trie
    }

    /// New suffix trie containing the suffixes of each sentence from
    /// the given file
    fn from_file(filename: &str) -> Result<SuffixTrie, io::Error> {
        let contents = fs::read_to_string(filename)?;
        let sentences: Vec<&str> = contents.split(".").collect();

        let mut suffix_trie = SuffixTrie::empty();
        for sentence in sentences {
            suffix_trie.add_string_suffixes(sentence);
        }
        Ok(suffix_trie)
    }

    /// Add the suffixes of a string to the suffix trie
    fn add_string_suffixes(&mut self, string: &str) {
        self.str_storage.push_str(string.clone());

        for (index, _c) in string.char_indices() {
            let suffix = &string[index..];
            self.add_suffix(suffix, index);
        }
    }

    fn add_suffix(&mut self, string: &str, string_key: usize) {
        let mut parent_index = 0;

        for c in string.chars() {
            let child_index = self.add_edge(c, parent_index);
            parent_index = child_index;
        }

        let parent: &mut SubTrie = self.get_node_mut(parent_index);
        parent.add_leaf_child(string_key);
    }

    fn add_node(&mut self, edge: char, parent_index: usize) -> usize {
        let child_index = self.node_storage.len();

        // Create empty child node
        self.node_storage.push(SubTrie::empty(child_index));

        // Add child index to parent's list of children
        self._unsafe_add_child_to_parent(edge, parent_index, child_index);

        // Return index of child node
        child_index
    }

    fn add_edge(&mut self, edge: char, parent_index: usize) -> usize {
        let parent = self.get_node(parent_index);
        let maybe_child_index: Option<&usize> = parent.get_child_index(edge);
        let child_index = match maybe_child_index {
            Some(index) => *index,
            None => {
                self.add_node(edge, parent_index)
            }
        };
        child_index
    }

    fn get_node(&self, node_index: usize) -> &SubTrie {
        let node = self.node_storage.get(node_index);
        match node {
            Some(n) =>  n,
            None => {
                panic!("Index out of bounds: {} size is {}", node_index, self.node_storage.len());
            }
        }
    }

    fn get_node_mut(&mut self, node_index: usize) -> &mut SubTrie {
        self.node_storage.get_mut(node_index).expect("Node not found!")
    }

    fn find_edit_distance(&self, pattern: &str, max_errors: usize) -> Vec<usize> {
        self.find_edit_distance_ignore(pattern, max_errors, HashMap::new())
    }

    fn find_edit_distance_ignore(&self,
                                 pattern: &str,
                                 max_errors: usize,
                                 ignored_characters: HashMap<char, bool>)
        -> Vec<usize> {
        let mut matcher = SuffixTrieEditMatcher::new(max_errors,
                                                 ignored_characters);
        matcher.find_edit_distance_ignore(&self, pattern)
    }


    /// Find all exact matches of the given pattern
    fn find_exact(&self, pattern: &str) -> Vec<usize> {
        let mut parent: &SubTrie = self.get_node(0);
        for c in pattern.chars() {
            let child = parent.get_child_index(c);
            match child {
                Some(child_index) => {
                    parent = self.get_node(*child_index);
                },
                None => return Vec::new()
            }
        }
        self.get_all_leaf_descendants(parent.node_index)
    }

    fn len(&self) -> usize {
        self.node_storage.len()
    }

    fn get_all_leaf_descendants(&self, node_index: usize) -> Vec<usize> {
        let mut leaves = Vec::new();
        let mut to_process: Vec<usize> = vec![node_index];
        while let Some(index) = to_process.pop() {
            let node = self.get_node(index);
            leaves.extend(&node.leaf_children);
            let children: Vec<usize> = node.children.values().cloned().collect();
            to_process.extend(&children);
        }
        leaves.sort();
        leaves
    }

    fn _unsafe_add_child_to_parent(&mut self,
                                   edge: char,
                                   parent_index: usize,
                                   child_index: usize) {
        // Shouldn't be called if the edge already exists
        let parent: &mut SubTrie = self.get_node_mut(parent_index);
        assert!(! parent.children.contains_key(&edge));
        parent.children.insert(edge, child_index);
    }
}

impl SubTrie {
    fn empty(node_index: usize) -> Self {
        SubTrie {
            children: HashMap::new(),
            node_index,
            leaf_children: vec![],
        }
    }

    fn get_child_index(&self, edge: char) -> Option<&usize> {
        self.children.get(&edge)
    }

    fn add_leaf_child(&mut self, key: usize) {
        self.leaf_children.push(key);
    }
}

#[derive(Clone,Debug)]
struct MatchesSet {
    indices: Vec<usize>,
    matches: HashMap<usize, Match>,
}

impl MatchesSet {
    fn empty() -> Self {
        MatchesSet {
            indices: vec![],
            matches: HashMap::new(),
        }
    }

    fn only_root_node() -> Self {
        let mut matches_set = MatchesSet::empty();
        matches_set.add_match(0, 0);
        matches_set
    }

    fn add_match(&mut self, index: usize, errors: usize) {
        let mut min_errors = errors;
        if let Some(existing_match) = self.matches.get(&index) {
            // We will reinsert this index with the minimum number of errors
            // we have found - there are multiple paths leading to the same
            // node
            min_errors = cmp::min(errors, existing_match.errors);
        } else {
            // This entry didn't already exist, add to vec of indices
            self.indices.push(index);
        }
        // Update the error count for this node
        let match_obj = Match::new(index, errors);
        self.matches.insert(index, match_obj);
    }

    fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

impl Iterator for MatchesSet {
    type Item = Match;

    fn next(&mut self) -> Option<Match> {
        let next_index = self.indices.pop();
        match next_index {
            Some(index) => {
                let match_obj = self.matches.remove(&index).expect("Corrupt MatchesSet object - no match object stored under index found in indices list");
                Some(match_obj)
            },
            None => None,
        }
    }
}

#[derive(Debug)]
struct SuffixTrieEditMatcher {
    matches_this_gen: MatchesSet,
    matches_next_gen: MatchesSet,
    ignored_characters: HashMap<char, bool>,
    max_errors: usize,
}

impl SuffixTrieEditMatcher {
    fn new(max_errors: usize,
           ignored_characters: HashMap<char, bool>) -> Self {
        SuffixTrieEditMatcher {
            matches_this_gen: MatchesSet::only_root_node(),
            matches_next_gen: MatchesSet::empty(),
            ignored_characters,
            max_errors,
        }
    }

    fn add_this_generation(&mut self, errors: usize, index: usize) {
        // Only add the match to the list if we haven't exceded the error limit
        if errors <= self.max_errors {
            self.matches_this_gen.add_match(index, errors);
        }
    }

    fn add_next_generation(&mut self, errors: usize, index: usize) {
        // Only add the match to the list if we haven't exceded the error limit
        if errors <= self.max_errors {
            self.matches_next_gen.add_match(index, errors);
        }
    }

    fn add_after_text_delete(&mut self, existing_errors: usize, index: usize) {
        println!("Adding node {} with errors {} - deletion from text", index, existing_errors + 1);
        self.add_this_generation(existing_errors + 1, index);
    }

    fn add_after_pattern_delete(&mut self, existing_errors: usize, index: usize) {
        println!("Adding node {} with errors {} - deletion from pattern", index, existing_errors + 1);
        self.add_next_generation(existing_errors + 1, index);
    }

    /// Process a possible match/mismatch between the current
    /// pattern character and the edge leading to this child
    /// If they match, or if either is in the set of ignorable characters,
    /// then don't increment the error. Otherwise, it is a mismatch and
    /// increases error by 1.
    fn add_after_mismatch(&mut self,
                          existing_errors: usize,
                          child_index: usize,
                          pattern_char: &char,
                          edge: &char) {
        let mut errors_after_match = existing_errors;
        if edge == pattern_char {
            // If the edge matches the character this doesn't add an error
        } else if self.ignored_characters.contains_key(edge) {
            // If the character is in the list of ignorable characters this doesn't add an error
        } else if self.ignored_characters.contains_key(pattern_char) {
            // If the character is in the list of ignorable characters this doesn't add an error
        } else {
            // Else this is a mismatch - increment the error counter
            errors_after_match += 1;
        }
        println!("Adding node {} with errors {} - match/mismatch", child_index, errors_after_match);
        self.add_next_generation(errors_after_match, child_index);
    }

    fn go_to_next_generation(&mut self) {
        self.matches_this_gen = self.matches_next_gen.clone();
        self.matches_next_gen = MatchesSet::empty();
    }

    fn find_edit_distance_ignore(&mut self,
                                 suffix_trie: &SuffixTrie,
                                 pattern: &str)
        -> Vec<usize> {

        // Keep track of matches and how many errors they have so far
        for c in pattern.chars() {
            println!("Matching char: {}", c);
            println!("Matching nodes: {:#?}", self);
            while let Some(parent_match) = self.matches_this_gen.next() {
                let parent = suffix_trie.get_node(parent_match.node_index);
                for (edge, child_index) in parent.children.iter() {
                    println!("Considering child {}", edge);
                    let existing_errors = parent_match.errors;
                    self.add_after_mismatch(existing_errors, *child_index, &c, &edge);
                    self.add_after_pattern_delete(existing_errors, parent_match.node_index);
                    self.add_after_text_delete(existing_errors, *child_index);
                }
                println!("Left this gen {:#?}", self.matches_this_gen);
                println!("Left next gen: {:#?}", self.matches_next_gen);
            }
            if self.matches_next_gen.is_empty() {
                // There are no partial matches
                return Vec::new();
            } else {
                self.go_to_next_generation();
            }
        }
        let mut leaves = vec![];
        while let Some(parent_match) = self.matches_this_gen.next() {
            let leaf_children = suffix_trie.get_all_leaf_descendants(parent_match.node_index);
            println!("Matching node: {:#?} with children {:#?}",
                     parent_match.node_index,
                     leaf_children);
            leaves.extend(leaf_children);
        }
        leaves.sort();
        leaves
    }
}

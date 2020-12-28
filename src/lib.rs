use std::cmp;
use std::cmp::Ordering;
use std::fs;
use std::io;
use std::io::{Error,ErrorKind};
use std::path::Path;
use std::collections::{HashMap,HashSet};

use bincode;
use log::{info,warn,debug,error};
use serde::{Serialize,Deserialize};

#[cfg(test)]
mod tests {
    use super::*;
    extern crate env_logger;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    fn compare_matches(matches: Vec<Match>, indices: Vec<usize>) {
        let match_indices: Vec<usize> = matches.iter().map(|l| l.index_in_str).collect();
        assert_eq!(match_indices, indices);
    }

    #[test]
    fn serialize_tests() {
        init();
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
        init();
        let trie = SuffixTrie::new("aba");
        println!("Result is {:#?}", trie);
        assert_eq!(trie.len(), 6);
    }

    #[test]
    fn test_leaves() {
        init();
        let trie = SuffixTrie::new("aba");
        println!("Result is {:#?}", trie);

        let expected: HashSet<usize> = (0..3).collect();
        // Gather together all leaf children from the SuffixTrie
        let mut actual: HashSet<usize> = HashSet::new();
        for node in trie.node_storage.iter() {
            for leaf_child in node.leaf_children.iter() {
                // Insert node to list, and assert that it wasn't already present
                assert!(actual.insert(leaf_child.index_in_str));
            }
        }
        // Check for equality
        assert!(actual.is_superset(&expected));
        assert!(expected.is_superset(&actual));
    }

    #[test]
    fn find_matches() {
        init();
        let trie = SuffixTrie::new("aba");
        println!("Result is {:#?}", trie);

        let matches = trie.find_exact("a");
        compare_matches(matches, vec![0, 2]);

        let trie = SuffixTrie::new("bananaBal");
        println!("Result is {:#?}", trie);

        let matches = trie.find_exact("an");
        compare_matches(matches, vec![1, 3]);

        let matches = trie.find_exact("ab");
        compare_matches(matches, vec![]);
    }

    #[test]
    fn find_matches_0_edit() {
        init();
        let trie = SuffixTrie::new("aba");
        println!("Result is {:#?}", trie);

        let matches = trie.find_edit_distance("a", 0);
        compare_matches(matches, vec![0, 2]);

        let trie = SuffixTrie::new("bananaBal");
        println!("Result is {:#?}", trie);

        let matches = trie.find_edit_distance("an", 0);
        compare_matches(matches, vec![1, 3]);

        let matches = trie.find_edit_distance("ab", 0);
        compare_matches(matches, vec![]);
    }

    #[test]
    fn find_matches_mismatch() {
        init();
        let trie = SuffixTrie::new("abcXef abXdef");
        println!("Result is {:#?}", trie);

        let matches = trie.find_edit_distance("abcdef", 1);
        compare_matches(matches, vec![0, 7]);
    }

    #[test]
    fn find_matches_insert_delete() {
        init();
        let trie = SuffixTrie::new("abcXdefg");
        println!("Result is {:#?}", trie);

        // Delete from text
        let matches = trie.find_edit_distance("abcdefg", 1);
        compare_matches(matches, vec![0]);

        // Delete from pattern
        let matches = trie.find_edit_distance("aXbc", 1);
        compare_matches(matches, vec![0]);
    }

    #[test]
    fn find_partial_matches_ignore() {
        init();
        let trie = SuffixTrie::new("He wracked wrack'd wrack'ed");
        println!("Result is {:#?}", trie);

        let mut ignored = HashMap::new();
        ignored.insert('e', true);
        ignored.insert('\'', true);
        let matches = trie.find_edit_distance_ignore("wrackd", 0, ignored.clone());
        compare_matches(matches, vec![3, 11, 19]);
        let matches = trie.find_edit_distance_ignore("wrack'de", 0, ignored.clone());
        compare_matches(matches, vec![3, 11, 19]);
    }

    fn find_single_wildcard() {
        init();
        let trie = SuffixTrie::new("oh this and that");
        println!("Result is {:#?}", trie);
        let matches = trie.find_edit_distance_ignore("th??", 0, HashMap::new());
        compare_matches(matches, vec![3, 12]);
    }

    #[test]
    fn find_matches_sentences() {
        init();
        let trie = SuffixTrie::from_file("resources/tests/small.txt").unwrap();
        println!("{:?}", trie.find_exact("Some"));
    }

    #[test]
    fn line_number_calculation() {
        init();
        let trie = SuffixTrie {
            line_start_indices: vec![0, 10, 20, 30],
            ..SuffixTrie::empty()
        };
        assert_eq!(trie.get_line_of_character(0), 0);
        assert_eq!(trie.get_line_of_character(1), 0);
        assert_eq!(trie.get_line_of_character(2), 0);
        assert_eq!(trie.get_line_of_character(9), 0);
        assert_eq!(trie.get_line_of_character(10), 1);
        assert_eq!(trie.get_line_of_character(29), 2);
        assert_eq!(trie.get_line_of_character(30), 3);
        assert_eq!(trie.get_line_of_character(31), 3);
        assert_eq!(trie.get_line_of_character(39), 3);
        assert_eq!(trie.get_line_of_character(139), 3);
    }

    #[test]
    fn construct_trie_from_file() {
        init();
        let trie = SuffixTrie::from_file("resources/tests/small.txt");
        println!("Result is {:#?}", trie);
        debug!("Test");
        match trie {
            Ok(trie) => println!("{:?}", trie.find_exact("drunken")),
            Err(e) => println!("{:#?}", e),
        }
    }

    fn bench_real_canon() {
        init();
        let trie = SuffixTrie::from_directory("resources/tests/large_1000/");
        match trie {
            Ok(trie) => println!("{:?}", trie.find_exact("love")),
            Err(e) => println!("{:#?}", e),
        }
    }

}

const SINGLE_WILDCARD: char = '?';

#[derive(Clone,Copy,Debug,Eq,Serialize,Deserialize)]
pub struct Match {
    text_index: usize,
    index_in_str: usize,
    start_line: usize,
    end_line: usize,
    length: usize,
    errors: usize,
}

impl Ord for Match {
    fn cmp(&self, other: &Self) -> Ordering {
        self.sort_key().cmp(&(other.sort_key()))
    }
}

impl PartialOrd for Match {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Match {
    fn eq(&self, other: &Self) -> bool {
        self.sort_key() == other.sort_key()
    }
}

impl Match {
    fn sort_key(&self) -> (usize, usize, usize, usize) {
        // Prefer matches which have fewer errors, are shorter, in earlier
        // texts and earlier within the text in which they appear
        (self.errors, self.length, self.text_index, self.index_in_str)
    }
}

#[derive(Clone,Copy,Debug,Eq,Serialize,Deserialize)]
struct Leaf {
    index_in_str: usize,
    text_index: usize,
}

impl Ord for Leaf {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.text_index, self.index_in_str).cmp(&(other.text_index, other.index_in_str))
    }
}

impl PartialOrd for Leaf {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Leaf {
    fn eq(&self, other: &Self) -> bool {
        (self.text_index, self.index_in_str) == (other.text_index, other.index_in_str)
    }
}

impl Leaf {
    fn new(index_in_str: usize, text_index: usize) -> Self {
        Leaf {
            index_in_str,
            text_index,
        }
    }
}

#[derive(Debug,Serialize,Deserialize)]
struct Text {
    name: String,
}

impl Text {
    fn new(name: &str) -> Self {
        Text {
            name: name.to_string(),
        }
    }
}

#[derive(Clone,Copy,Debug)]
struct WorkingMatch {
    node_index: usize,
    errors: usize,
    length: usize,
}

#[derive(Debug,Serialize,Deserialize)]
pub struct SuffixTrie {
    // Place to store entire string - keeps ownership simple
    str_storage: String,
    // Place to store all the nodes
    node_storage: Vec<SubTrie>,
    // Information about each of the texts (e.g. files) included in
    // the Suffix Trie
    texts: Vec<Text>,
    // Indices of the starts of lines
    line_start_indices: Vec<usize>,
}

#[derive(Debug,Serialize,Deserialize)]
struct SubTrie {
    // Index of this node in the overall array
    node_index: usize,
    // List of children node indices, indexed by the character labelling the edge
    // from the parent to the child
    children: HashMap<char, usize>,
    // List of indices at which this suffix is present
    leaf_children: Vec<Leaf>,
}

impl WorkingMatch {
    fn new(node_index: usize, errors: usize, length: usize) -> Self {
        WorkingMatch {
            node_index,
            errors,
            length,
        }
    }
}

impl SuffixTrie {
    /// New suffix trie containing suffixes of a single string
    pub fn new(string: &str) -> Self {
        let mut suffix_trie = SuffixTrie::empty();
        suffix_trie.texts.push(Text::new("first text"));
        suffix_trie.add_string_suffixes(string, 0, 0);
        suffix_trie
    }

    /// New empty suffix trie
    pub fn empty() -> Self {
        let root_node = SubTrie::empty(0);
        let mut suffix_trie = SuffixTrie {
            str_storage: String::from(""),
            node_storage: vec![root_node],
            texts: vec![],
            line_start_indices: vec![0],
        };
        suffix_trie
    }

    /// New suffix trie containing the suffixes of each sentence from
    /// the given file
    pub fn from_file(path: &str) -> Result<SuffixTrie, io::Error> {
        let mut suffix_trie = SuffixTrie::empty();
        suffix_trie.add_file(path)?;
        Ok(suffix_trie)
    }

    pub fn add_file(&mut self, path: &str) -> Result<(), io::Error> {
        let contents = fs::read_to_string(path)?;
        let sentences: Vec<&str> = contents.split(".").collect();

        self.texts.push(Text::new(path));
        let text_index = self.texts.len() - 1;

        let mut sentence_start = 0;
        for sentence in sentences {
            self.add_string_suffixes(sentence, sentence_start, text_index);
            sentence_start += sentence.len();
        }
        Ok(())
    }

    /// New suffix trie containing the suffixes of each sentence from
    /// each file in the given directory
    pub fn from_directory(path: &str) -> Result<SuffixTrie, io::Error> {
        let mut suffix_trie = SuffixTrie::empty();

        let files = fs::read_dir(path)?;
        for file in files {
            info!("Attempting to read file {:?}", file);
            let file = file?;
            match file.path().to_str() {
                Some(path) => suffix_trie.add_file(path)?,
                None => return Err(Error::new(ErrorKind::InvalidInput,
                                              "Failed to convert path to string")),
            }
        }
        Ok(suffix_trie)
    }

    /// Add the suffixes of a string to the suffix trie
    fn add_string_suffixes(&mut self,
                           string: &str,
                           start_index: usize,
                           text_index: usize) {
        self.str_storage.push_str(string.clone());

        for (index, c) in string.char_indices() {
            if c == '\n' {
                self.line_start_indices.push(index + start_index);
                debug!("Adding line to line_start_indices {:?}", self.line_start_indices);
            }

            let suffix = &string[index..];
            let total_index = start_index + index;
            self.add_suffix(suffix, total_index, text_index);
        }
    }

    fn add_suffix(&mut self, string: &str,
                  index_in_text: usize,
                  text_index: usize) {
        let mut parent_index = 0;

        for c in string.chars() {
            let child_index = self.add_edge(c, parent_index);
            parent_index = child_index;
        }

        let parent: &mut SubTrie = self.get_node_mut(parent_index);
        parent.add_leaf_child(Leaf::new(index_in_text, text_index));
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

    pub fn find_edit_distance(&self, pattern: &str, max_errors: usize) -> Vec<Match> {
        self.find_edit_distance_ignore(pattern, max_errors, HashMap::new())
    }

    pub fn find_edit_distance_ignore(&self,
                                 pattern: &str,
                                 max_errors: usize,
                                 ignored_characters: HashMap<char, bool>)
        -> Vec<Match> {
        let mut matcher = SuffixTrieEditMatcher::new(max_errors,
                                                 ignored_characters);
        matcher.find_edit_distance_ignore(&self, pattern)
    }

    /// Find all exact matches of the given pattern
    pub fn find_exact(&self, pattern: &str) -> Vec<Match> {
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
        let leaves = self.get_all_leaf_descendants(parent.node_index);
        let mut matches = self.match_array_from_leaves(leaves, pattern.len(), 0);
        matches.sort();
        matches
    }

    fn len(&self) -> usize {
        self.node_storage.len()
    }

    fn get_all_leaf_descendants(&self, node_index: usize) -> Vec<Leaf> {
        let mut leaves = Vec::new();
        let mut to_process: Vec<usize> = vec![node_index];
        while let Some(index) = to_process.pop() {
            let node = self.get_node(index);
            leaves.extend(&node.leaf_children);
            let children: Vec<usize> = node.children.values().cloned().collect();
            to_process.extend(&children);
        }
        leaves.sort();
        leaves.clone()
    }

    fn match_array_from_leaves(&self,
                               leaves: Vec<Leaf>,
                               length: usize,
                               errors: usize) -> Vec<Match> {
        let mut matches = vec![];

        for leaf in leaves.iter() {
            let (start_line, end_line) = self.get_lines_of_substring(leaf.index_in_str,
                                                                     length);
            let match_obj = Match {
                text_index: leaf.text_index,
                index_in_str: leaf.index_in_str,
                start_line,
                end_line,
                length,
                errors,
            };
            matches.push(match_obj);
        }

        matches
    }

    fn char_before_line(&self, char_index: usize, line_index: usize) -> bool {
        let mut is_before_line;
        if line_index == self.line_start_indices.len() {
            // This is an invalid line index (too high) so the character
            // must come on a line before this one
            is_before_line = true;
        } else if char_index < self.line_start_indices[line_index] {
            // The character index is before the index of the start of this
            // line, so the character comes before the line
            is_before_line = true;
        } else {
            // Character index after the index of start of line, so character
            // is on this line or afterwards
            is_before_line = false;
        }
        is_before_line
    }

    fn get_line_of_character(&self, char_index: usize) -> usize {
        // Find the last line_index smaller than char_index
        let mut found = false;
        let last_line = self.line_start_indices.len();
        let mut lower_line_limit = 0;
        let mut upper_line_limit = match last_line {
            0 => 0,
            ll => ll,
        };
        debug!("Finding index of line containing char index {}", char_index);
        let mut current_line: usize = (upper_line_limit - lower_line_limit)/2;
        while !found && lower_line_limit != upper_line_limit {
            assert!(lower_line_limit <= current_line);
            assert!(upper_line_limit >= current_line);
            debug!("Upper: {}, Lower: {}, Current: {}", upper_line_limit, lower_line_limit, current_line);
            if self.char_before_line(char_index, current_line) {
                // The character must be on an earlier line
                upper_line_limit = cmp::max(current_line - 1, 0);
            } else {
                // The character is on this line or later
                if self.char_before_line(char_index, current_line + 1) {
                    // It must be on the current line, since it can't be later
                    // (it's before the next line)
                    found = true;
                } else {
                    // The character is on a later line
                    lower_line_limit = cmp::min(current_line + 1, last_line);
                }

            }
            current_line = (upper_line_limit - lower_line_limit)/2;
        }
        current_line
    }

    /// Find the index of the line where this substring starts and the index
    /// of the line where it ends
    fn get_lines_of_substring(&self, start_index: usize, length: usize) -> (usize, usize) {
        let start_line = self.get_line_of_character(start_index);
        let end_line = self.get_line_of_character(start_index + length);

        (start_line, end_line)
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

    fn add_leaf_child(&mut self, key: Leaf) {
        self.leaf_children.push(key);
    }
}

#[derive(Clone,Debug)]
struct WorkingMatchesSet {
    indices: Vec<usize>,
    working_matches: HashMap<usize, WorkingMatch>,
}

impl WorkingMatchesSet {
    fn empty() -> Self {
        WorkingMatchesSet {
            indices: vec![],
            working_matches: HashMap::new(),
        }
    }

    fn only_root_node() -> Self {
        let mut working_matches_set = WorkingMatchesSet::empty();
        working_matches_set.add_working_match(0, 0, 0);
        working_matches_set
    }

    fn add_working_match(&mut self, index: usize, errors: usize, length: usize) {
        let mut min_errors = errors;
        if let Some(existing_match) = self.working_matches.get(&index) {
            // We will reinsert this index with the minimum number of errors
            // we have found - there are multiple paths leading to the same
            // node
            debug!("Updating! existing match is {:?} but we now have one with length {} and errors {}", existing_match, length, errors);
            min_errors = cmp::min(errors, existing_match.errors);
        } else {
            // This entry didn't already exist, add to vec of indices
            self.indices.push(index);
        }
        // Update the error count for this node
        let match_obj = WorkingMatch::new(index, min_errors, length);
        self.working_matches.insert(index, match_obj);
    }

    fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

impl Iterator for WorkingMatchesSet {
    type Item = WorkingMatch;

    fn next(&mut self) -> Option<WorkingMatch> {
        let next_index = self.indices.pop();
        match next_index {
            Some(index) => {
                let match_obj = self.working_matches.remove(&index).expect("Corrupt WorkingMatchesSet object - no match object stored under index found in indices list");
                Some(match_obj)
            },
            None => None,
        }
    }
}

#[derive(Debug)]
struct SuffixTrieEditMatcher {
    matches_this_gen: WorkingMatchesSet,
    matches_next_gen: WorkingMatchesSet,
    ignored_characters: HashMap<char, bool>,
    max_errors: usize,
}

impl SuffixTrieEditMatcher {
    fn new(max_errors: usize,
           ignored_characters: HashMap<char, bool>) -> Self {
        SuffixTrieEditMatcher {
            matches_this_gen: WorkingMatchesSet::only_root_node(),
            matches_next_gen: WorkingMatchesSet::empty(),
            ignored_characters,
            max_errors,
        }
    }

    fn add_this_generation(&mut self, errors: usize, index: usize, length: usize) {
        // Only add the match to the list if we haven't exceded the error limit
        if errors <= self.max_errors {
            self.matches_this_gen.add_working_match(index, errors, length);
        }
    }

    fn add_next_generation(&mut self, errors: usize, index: usize, length: usize) {
        // Only add the match to the list if we haven't exceded the error limit
        if errors <= self.max_errors {
            self.matches_next_gen.add_working_match(index, errors, length);
        }
    }

    fn add_after_pattern_delete(&mut self, existing_match: WorkingMatch) {
        self.add_next_generation(existing_match.errors + 1,
                                 existing_match.node_index,
                                 existing_match.length);
    }

    fn add_after_text_delete(&mut self,
                             existing_match: WorkingMatch,
                             child_index: usize) {
        self.add_this_generation(existing_match.errors + 1,
                                 child_index,
                                 existing_match.length + 1);
    }

    /// Process a possible match/mismatch between the current
    /// pattern character and the edge leading to this child
    /// If they match, or if either is in the set of ignorable characters,
    /// then don't increment the error. Otherwise, it is a mismatch and
    /// increases error by 1.
    fn add_after_mismatch(&mut self,
                          existing_match: WorkingMatch,
                          child_index: usize,
                          pattern_char: &char,
                          edge: &char) {
        let mut errors_after_match = existing_match.errors;
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
        debug!("Adding node {} with errors {} - match/mismatch", child_index, errors_after_match);
        self.add_next_generation(errors_after_match,
                                 child_index,
                                 existing_match.length + 1);
    }

    fn go_to_next_generation(&mut self) {
        self.matches_this_gen = self.matches_next_gen.clone();
        self.matches_next_gen = WorkingMatchesSet::empty();
    }

    fn find_edit_distance_ignore(&mut self,
                                 suffix_trie: &SuffixTrie,
                                 pattern: &str)
        -> Vec<Match> {

        // Keep track of matches and how many errors they have so far
        for c in pattern.chars() {
            debug!("Matching char: {}", c);
            debug!("Matching nodes: {:#?}", self);
            while let Some(parent_match) = self.matches_this_gen.next() {
                debug!("Parent match: {:?}", parent_match);
                let parent = suffix_trie.get_node(parent_match.node_index);
                for (edge, child_index) in parent.children.iter() {
                    debug!("Considering child {}", edge);
                    self.add_after_mismatch(parent_match,
                                            *child_index,
                                            &c,
                                            &edge);
                    self.add_after_pattern_delete(parent_match);
                    self.add_after_text_delete(parent_match,
                                               *child_index);
                }
                debug!("Left this gen {:#?}", self.matches_this_gen);
                debug!("Left next gen: {:#?}", self.matches_next_gen);
            }
            if self.matches_next_gen.is_empty() {
                // There are no partial matches
                return Vec::new();
            } else {
                self.go_to_next_generation();
            }
        }
        let mut matches = vec![];
        while let Some(parent_match) = self.matches_this_gen.next() {
            let leaf_children = suffix_trie.get_all_leaf_descendants(parent_match.node_index);
            debug!("Matching node: {:#?} with children {:#?}",
                   parent_match.node_index,
                   leaf_children);
            let parent_matches = suffix_trie.match_array_from_leaves(leaf_children,
                                                                     parent_match.length,
                                                                     parent_match.errors);
            matches.extend(parent_matches);
        }
        matches.sort();
        matches
    }
}

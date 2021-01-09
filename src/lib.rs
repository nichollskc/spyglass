use std::cmp;
use std::cmp::Ordering;
use std::fs;
use std::io;
use std::io::{Error,ErrorKind};
use std::collections::HashMap;
use std::str::Chars;

use deunicode;
use log::{info,warn,debug,error};
use serde::{Serialize,Deserialize};

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashSet;
    use utilities;

    #[test]
    fn test_build_size() {
        utilities::init_testing();
        let mut trie = SuffixTrie::empty();
        trie.add_sentences_from_text("test", "ABCDE.<<STOP>>ABCDE.<<STOP>>ABCDE.");
        println!("Result is {:#?}", trie);
        assert_eq!(trie.len(), 7);
        trie.add_sentences_from_text("duplicate", "ABCDE.<<STOP>>ABCDE.<<STOP>>ABCDE.");
        println!("Result is {:#?}", trie);
        assert_eq!(trie.len(), 7);

        let trie = SuffixTrie::new("abcabdabe");
        println!("Result is {:#?}", trie);
        assert_eq!(trie.len(), 12);
    }

    #[test]
    fn test_build_leaves() {
        utilities::init_testing();
        helper_test_leaves("abcdefghijk");
        helper_test_leaves("ababacababccbabcbabccbabcbababcbcbabcbbacbcbabcab");
    }

    fn helper_test_leaves(string: &str) {
        let trie = SuffixTrie::new(string);
        println!("Result is {:#?}", trie);

        let expected: HashSet<usize> = (0..string.len()).collect();
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
    fn line_number_calculation() {
        utilities::init_testing();
        let text = Text {
            line_start_indices: vec![0, 10, 20, 30, 40, 50, 60, 70],
            ..Text::new("noname", 0)
        };
        for index in 0..9 {
            for line in 0..7 {
                assert_eq!(text.get_line_of_character(index + line*10), line);
            }
            for line in 8..10 {
                assert_eq!(text.get_line_of_character(index + line*10), 7);
            }
        }
        assert_eq!(text.get_line_of_character(0), 0);
        assert_eq!(text.get_line_of_character(1), 0);
        assert_eq!(text.get_line_of_character(2), 0);
        assert_eq!(text.get_line_of_character(9), 0);
        assert_eq!(text.get_line_of_character(10), 1);
        assert_eq!(text.get_line_of_character(29), 2);
        assert_eq!(text.get_line_of_character(30), 3);
        assert_eq!(text.get_line_of_character(31), 3);
        assert_eq!(text.get_line_of_character(139), 7);
    }

}

const SINGLE_WILDCARD: char = '?';

#[derive(Clone,Debug,Eq,Serialize,Deserialize)]
pub struct Match {
    pub text_index: usize,
    pub index_in_str: usize,
    pub start_line: usize,
    pub end_line: usize,
    pub length: usize,
    pub errors: usize,
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
    // Indices of the starts of lines
    line_start_indices: Vec<usize>,
    last_index: usize,
    offset: usize,
}

impl Text {
    fn new(name: &str, offset: usize) -> Self {
        Text {
            name: name.to_string(),
            line_start_indices: vec![0],
            last_index: 0,
            offset,
        }
    }

    fn char_before_line(&self, char_index: usize, line_index: usize) -> bool {
        let is_before_line;
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
            ll => ll - 1,
        };
        debug!("Finding index of line containing char index {}", char_index);
        debug!("Line start indices are {:?}", self.line_start_indices);
        let mut current_line: usize = (upper_line_limit - lower_line_limit)/2;
        while !found && lower_line_limit != upper_line_limit {
            debug!("Upper: {}, Lower: {}, Current: {}", upper_line_limit, lower_line_limit, current_line);
            assert!(lower_line_limit <= current_line);
            assert!(upper_line_limit >= current_line);
            if self.char_before_line(char_index, current_line) {
                // The character must be on an earlier line
                upper_line_limit = cmp::max(current_line - 1, 0);
            } else {
                // The character is on this line or later
                if self.char_before_line(char_index, current_line + 1) {
                    // It must be on the current line, since it can't be later
                    // (it's before the next line)
                    found = true;
                    debug!("Found matching line: {}", current_line);
                } else {
                    // The character is on a later line
                    lower_line_limit = cmp::min(current_line + 1, last_line);
                }

            }
            current_line = lower_line_limit + (upper_line_limit - lower_line_limit)/2;
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
}

enum EdgeMatchKind {
    WholeMatch,
    EarlyStop,
    Diverge(char),
}

struct EdgeMatch {
    overlap_type: EdgeMatchKind,
    shared_length: usize,
}

#[derive(Clone,Copy,Debug,Eq,Hash)]
struct CharLocation {
    node_index: usize,
    index_in_edge: usize,
}

impl Ord for CharLocation {
    fn cmp(&self, other: &Self) -> Ordering {
        self.sort_key().cmp(&(other.sort_key()))
    }
}

impl PartialOrd for CharLocation {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for CharLocation {
    fn eq(&self, other: &Self) -> bool {
        self.sort_key() == other.sort_key()
    }
}

impl CharLocation {
    fn sort_key(&self) -> (usize, usize) {
        // Prefer matches which have fewer errors, are shorter, in earlier
        // texts and earlier within the text in which they appear
        (self.node_index, self.index_in_edge)
    }
}

#[derive(Clone,Copy,Debug)]
struct WorkingMatch {
    starting_char: CharLocation,
    errors: usize,
    length: usize,
}

#[derive(Debug,Serialize,Deserialize)]
pub struct SuffixTrie {
    // Place to store entire string - keeps ownership simple
    str_storage: Vec<char>,
    // Place to store all the nodes
    node_storage: Vec<SubTrie>,
    // Information about each of the texts (e.g. files) included in
    // the Suffix Trie
    texts: Vec<Text>,
}

#[derive(Debug,Serialize,Deserialize)]
struct SubTrie {
    // Index of this node in the overall array
    node_index: usize,
    // List of children node indices, indexed by the string labelling the edge
    // from the parent to the child. The key is the first character of the edge.
    children: HashMap<char, usize>,
    // List of indices at which this suffix is present
    leaf_children: Vec<Leaf>,
    // Index where the string labelling the edge from this node's parent starts
    // and the length of this edge.
    edge_start_index: usize,
    edge_length: usize,
}

impl WorkingMatch {
    fn new(starting_char: CharLocation, errors: usize, length: usize) -> Self {
        WorkingMatch {
            starting_char,
            errors,
            length,
        }
    }
}

impl SuffixTrie {
    /// New suffix trie containing suffixes of a single string
    pub fn new(string: &str) -> Self {
        let mut suffix_trie = SuffixTrie::empty();
        suffix_trie.texts.push(Text::new("first text", 0));
        suffix_trie.add_string_suffixes(string, 0, 0);
        suffix_trie
    }

    /// New empty suffix trie
    pub fn empty() -> Self {
        let root_node = SubTrie::empty(0, 0, 0);
        let suffix_trie = SuffixTrie {
            str_storage: vec![],
            node_storage: vec![root_node],
            texts: vec![],
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
        self.add_sentences_from_text(path, &contents);
        Ok(())
    }


    pub fn add_sentences_from_text(&mut self, text_name: &str, contents: &str) {
        let sentences: Vec<&str> = contents.split("<<STOP>>").collect();

        let offset = self.str_storage.len();
        self.texts.push(Text::new(text_name, offset));
        let text_index = self.texts.len() - 1;

        let mut sentence_start = 0;
        for sentence in sentences {
            let num_chars = self.add_string_suffixes(sentence, sentence_start, text_index);
            sentence_start += num_chars;
        }
    }

    /// New suffix trie containing the suffixes of each sentence from
    /// each file in the given directory
    pub fn from_directory(path: &str) -> Result<SuffixTrie, io::Error> {
        let mut suffix_trie = SuffixTrie::empty();

        let files = fs::read_dir(path)?;
        let mut paths: Vec<String> = vec![];

        for file in files {
            info!("Attempting to read file {:?}", file);
            let file = file?;
            let path = file.path();
            match path.to_str() {
                Some(path_str) => paths.push(path_str.to_string()),
                None => return Err(Error::new(ErrorKind::InvalidInput,
                                              "Failed to convert path to string")),
            }
        }
        paths.sort();

        for path in paths {
            suffix_trie.add_file(&path)?
        }

        Ok(suffix_trie)
    }

    /// Add the suffixes of a string to the suffix trie
    fn add_string_suffixes(&mut self,
                           string: &str,
                           start_index: usize,
                           text_index: usize) -> usize{
        let mut num_chars = 0;
        let ascii_string = deunicode::deunicode(string);
        self.str_storage.extend(ascii_string.chars());

        for (index, c) in ascii_string.char_indices() {
            num_chars += 1;
            if c == '\n' {
                self.texts[text_index].line_start_indices.push(index + start_index + 1);
                debug!("Adding line to line_start_indices {:?}", self.texts[text_index].line_start_indices);
            }

            let suffix = &ascii_string[index..];
            let total_index = start_index + index;
            self.add_suffix(suffix, total_index, text_index);
        }
        self.texts[text_index].last_index += num_chars;
        num_chars
    }

    fn add_suffix(&mut self,
                  string: &str,
                  index_in_text: usize,
                  text_index: usize) {
        let mut parent_index = 0;
        let mut child_index = 0;
        let mut string_iterator = string.chars();
        let mut current_char_index = index_in_text + self.texts[text_index].offset;
        debug!("Adding suffix {} to tree {:#?}", string, self);
        while let Some(c) = &string_iterator.next() {
            // Check if there is an edge starting with this char in the parent
            let parent: &SubTrie = self.get_node(parent_index);
            debug!("Looking to add character {} to trie. Parent is {}", *c, parent_index);
            if let Some(ancestor_index) = parent.get_child_index(*c) {
                // There is an existing node starting with this character
                debug!("Found existing parent {}. Will add this suffix below this node.", ancestor_index);
                let (cci, ci) = self.insert_within_edge(*ancestor_index,
                                                        &mut string_iterator,
                                                        current_char_index);
                current_char_index = cci;
                child_index = ci;
            } else {
                // There is no edge, simply add a edge from this parent
                // labelled with the rest of the string
                debug!("No existing parent");
                child_index = self.add_node(parent_index,
                                            *c,
                                            current_char_index,
                                            &string_iterator.count() + 1);
                break;
            }
            parent_index = child_index;
        }

        debug!("Adding leaf for index_in_text {} to node {}", index_in_text, child_index);
        let final_node: &mut SubTrie = self.get_node_mut(child_index);
        final_node.add_leaf_child(Leaf::new(index_in_text, text_index));
    }

    /// Split the edge from the given node into two, the first part having length
    /// first_length.
    ///
    /// Currently:  L
    /// grandparent -> parent (-> children)
    ///
    /// Want:       X         Y
    /// grandparent -> parent -> new (-> children)
    /// I.e. the edge from grandparent to parent is now split into two, with
    /// edge lengths X and Y, so that X+Y=L (original length) and X=first_length.
    fn split_edge(&mut self, node_index: usize, first_length: usize) {
        debug!("Splitting edge of {}. Edge to this node will have length {}", node_index, first_length);
        let mut parent = self.get_node_mut(node_index);
        let new_edge_start_index = parent.edge_start_index + first_length;
        let new_edge_length = parent.edge_length - first_length;
        // We are splitting the edge into two new edges, so the new
        // length must be shorter
        assert!(parent.edge_length > first_length);

        parent.edge_length = first_length;
        // Extract existing children from this parent, we will add them to
        // the new node.
        let children: HashMap<char, usize> = parent.children.drain().collect();
        let leaf_children: Vec<Leaf> = parent.leaf_children.drain(..).collect();

        let edge = self.str_storage[new_edge_start_index].clone();
        let new_node_index = self.add_node(node_index,
                                           edge,
                                           new_edge_start_index,
                                           new_edge_length);
        let new_node = self.get_node_mut(new_node_index);
        new_node.children = children;
        new_node.leaf_children = leaf_children;
    }


    fn add_node(&mut self,
                parent_index: usize,
                edge: char,
                char_index: usize,
                edge_length: usize) -> usize {
        let child_index = self.node_storage.len();
        debug!("Adding node {} to parent {} with edge {}, edge_start_index {} and edge_length {}",
               child_index, parent_index, edge, char_index, edge_length);

        // Create empty child node
        self.node_storage.push(SubTrie::empty(child_index,
                                              char_index,
                                              edge_length));

        // Edge should match the value at the given index in the string
        assert_eq!(edge, self.str_storage[char_index]);

        // Add child index to parent's list of children
        self._unsafe_add_child_to_parent(edge,
                                         parent_index,
                                         child_index);

        // Shouldn't be called if the edge already exists
        // Return index of child node
        child_index
    }

    fn consume_all_shared_length(&self,
                                 parent_index: usize,
                                 string_iterator: &mut Chars,
                                 config: &MatcherConfig) -> EdgeMatch {
        let ancestor = self.get_node(parent_index);
        let ancestor_start = ancestor.edge_start_index;
        let ancestor_length = ancestor.edge_length;

        let mut edge_match = EdgeMatch {
            overlap_type: EdgeMatchKind::WholeMatch,
            shared_length: ancestor_length,
        };

        // Run through character by character until we find the place
        // where these strings diverge
        // Start at the second character of the existing edge, and the next
        // character of our edge
        let mut index_in_edge = 1;
        let mut edges_agree = true;
        while index_in_edge < ancestor_length && edges_agree {
            // Get next character of our string and compare to next
            // character of existing edge
            if let Some(c) = string_iterator.next() {
                let index = ancestor_start + index_in_edge;
                let ancestor_c = self.str_storage[index];
                debug!("Next character of suffix is {}, next ancestor character is {}", c, ancestor_c);

                if ! config.chars_match(&c, &ancestor_c) {
                    edge_match = EdgeMatch {
                        overlap_type: EdgeMatchKind::Diverge(c),
                        shared_length: index_in_edge,
                    };
                    edges_agree = false
                }
            } else {
                edge_match = EdgeMatch {
                    overlap_type: EdgeMatchKind::EarlyStop,
                    shared_length: index_in_edge,
                };
                edges_agree = false;
            }
            index_in_edge += 1;
        }

        edge_match
    }

    fn insert_within_edge(&mut self,
                          parent_index: usize,
                          string_iterator: &mut Chars,
                          start_index: usize) -> (usize, usize) {
        let edge_match = self.consume_all_shared_length(parent_index,
                                                        string_iterator,
                                                        &MatcherConfig::exact());

        debug!("Shared length with ancestor edge was {}", edge_match.shared_length);

        let child_index = match edge_match.overlap_type {
            EdgeMatchKind::WholeMatch =>  {
                debug!("Entire ancestor edge matched with our string. No changes to this ancestor needed. Any remaining characters will be added below this node.");
                parent_index
            },
            EdgeMatchKind::Diverge(last_char) => {
                debug!("Ancestor edge and our string diverge. Splitting ancestor edge here to add node for rest of suffix here");
                self.split_edge(parent_index,
                                edge_match.shared_length);
                self.add_node(parent_index,
                              last_char,
                              start_index + edge_match.shared_length,
                              string_iterator.count() + 1)
            },
            EdgeMatchKind::EarlyStop => {
                debug!("More characters in ancestor edge than our string. Splitting edge to add leaf in middle of ancestor edge.");
                self.split_edge(parent_index,
                                edge_match.shared_length);
                parent_index
            },
        };

        (start_index + edge_match.shared_length, child_index)
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
        self.find_edit_distance_ignore(pattern, max_errors, HashMap::new(), false)
    }

    pub fn find_edit_distance_ignore(&self,
                                     pattern: &str,
                                     max_errors: usize,
                                     ignored_characters: HashMap<char, bool>,
                                     case_insensitive: bool)
        -> Vec<Match> {
            let config = MatcherConfig {
                max_errors,
                ignored_characters,
                case_insensitive,
            };
            let mut matcher = SuffixTrieEditMatcher::new(config);
            matcher.find_edit_distance_ignore(&self, pattern)
        }

    /// Find all exact matches of the given pattern
    pub fn find_exact(&self, pattern: &str, case_insensitive: bool) -> Vec<Match> {
        let empty_config = MatcherConfig {
            case_insensitive,
            ..MatcherConfig::exact()
        };
        let mut matcher = SuffixTrieEditMatcher::new(empty_config);
        matcher.find_exact(&self, pattern)
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
            let text = &self.texts[leaf.text_index];
            let (start_line, end_line) = text.get_lines_of_substring(leaf.index_in_str,
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

    fn owned_lines_after(&self,
                         text: &Text,
                         line_index: usize,
                         lines_after: usize,
                         start_char_index: usize) -> String {
        let end_line = line_index + lines_after;
        let end_char_index = if end_line + 2 >= text.line_start_indices.len() {
            // This is either beyond the end of the text, or is the very last
            // line. We must return the end of the text
            text.last_index
        } else {
            text.line_start_indices[end_line + 1]
        };
        let length = end_char_index - start_char_index;
        self.owned_from_index(text, start_char_index, length)
    }

    fn owned_lines_before(&self,
                          text: &Text,
                          line_index: usize,
                          lines_before: usize,
                          end_char_index: usize) -> String {
        let start_char_index = if lines_before > line_index {
            0
        } else {
            let start_line = line_index - lines_before;
            text.line_start_indices[start_line]
        };
        let length = end_char_index - start_char_index;
        self.owned_from_index(text, start_char_index, length)
    }

    fn owned_from_index(&self,
                        text: &Text,
                        index_in_str: usize,
                        length: usize) -> String {
        let start = index_in_str + text.offset;
        let end = start + length;
        (self.str_storage[start .. end]).iter().cloned().collect::<String>()
    }

    pub fn get_strings_of_match(&self,
                                match_obj: &Match,
                                context_lines: usize) -> (String, String, String) {
        let text = &self.texts[match_obj.text_index];
        let matching = self.owned_from_index(text,
                                             match_obj.index_in_str,
                                             match_obj.length);
        let before = self.owned_lines_before(text,
                                             match_obj.start_line,
                                             context_lines,
                                             match_obj.index_in_str);
        let after = self.owned_lines_after(text,
                                           match_obj.end_line,
                                           context_lines,
                                           match_obj.index_in_str + match_obj.length);
        (before, matching, after)
    }

    fn _unsafe_add_child_to_parent(&mut self,
                                   edge: char,
                                   parent_index: usize,
                                   child_index: usize) {
        let parent: &mut SubTrie = self.get_node_mut(parent_index);
        // Shouldn't be called if the edge already exists
        assert!(! parent.children.contains_key(&edge));

        parent.children.insert(edge, child_index);
    }

    pub fn get_text_names(&self) -> Vec<String> {
        let mut text_names: Vec<String> = vec![];
        for text in self.texts.iter() {
            text_names.push(text.name.to_string());
        }
        text_names
    }
}

impl SubTrie {
    fn empty(node_index: usize,
             edge_start_index: usize,
             edge_length: usize) -> Self {
        SubTrie {
            children: HashMap::new(),
            node_index,
            leaf_children: vec![],
            edge_start_index,
            edge_length,
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
    indices: Vec<CharLocation>,
    working_matches: HashMap<CharLocation, WorkingMatch>,
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
        let root_location = CharLocation {
            node_index: 0,
            index_in_edge: 0,
        };
        working_matches_set.add_working_match(root_location, 0, 0);
        working_matches_set
    }

    fn add_working_match(&mut self, starting_char: CharLocation, errors: usize, length: usize) {
        let mut min_errors = errors;
        if let Some(existing_match) = self.working_matches.get(&starting_char) {
            // We will reinsert this index with the minimum number of errors
            // we have found - there are multiple paths leading to the same
            // node
            debug!("Updating! existing match is {:?} but we now have one with length {} and errors {}", existing_match, length, errors);
            min_errors = cmp::min(errors, existing_match.errors);
        } else {
            // This entry didn't already exist, add to vec of indices
            self.indices.push(starting_char);
        }
        // Update the error count for this node
        let match_obj = WorkingMatch::new(starting_char, min_errors, length);
        self.working_matches.insert(starting_char, match_obj);
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
pub struct MatcherConfig {
    ignored_characters: HashMap<char, bool>,
    max_errors: usize,
    case_insensitive: bool,
}

impl MatcherConfig {
    fn exact() -> Self {
        MatcherConfig {
            ignored_characters: HashMap::new(),
            max_errors: 0,
            case_insensitive: false
        }
    }

    fn chars_match(&self, char1: &char, char2: &char) -> bool {
        let mut result = false;
        if char1 == char2 {
            result = true;
        } else if self.case_insensitive {
            if char1.to_ascii_lowercase() == char2.to_ascii_lowercase() {
                result = true;
            }
        } else if self.ignored_characters.contains_key(char1) {
            // If the character is in the list of ignorable characters this doesn't add an error
            result = true;
        } else if self.ignored_characters.contains_key(char2) {
            // If the character is in the list of ignorable characters this doesn't add an error
            result = true;
        }
        result
    }
}

#[derive(Debug)]
struct SuffixTrieEditMatcher {
    matches_this_gen: WorkingMatchesSet,
    matches_next_gen: WorkingMatchesSet,
    config: MatcherConfig,
}

impl SuffixTrieEditMatcher {
    fn new(config: MatcherConfig) -> Self {
        SuffixTrieEditMatcher {
            matches_this_gen: WorkingMatchesSet::only_root_node(),
            matches_next_gen: WorkingMatchesSet::empty(),
            config,
        }
    }

    fn add_this_generation(&mut self, errors: usize, location: CharLocation, length: usize) {
        // Only add the match to the list if we haven't exceded the error limit
        if errors <= self.config.max_errors {
            self.matches_this_gen.add_working_match(location, errors, length);
        }
    }

    fn add_next_generation(&mut self, errors: usize, location: CharLocation, length: usize) {
        // Only add the match to the list if we haven't exceded the error limit
        if errors <= self.config.max_errors {
            self.matches_next_gen.add_working_match(location, errors, length);
        }
    }

    fn add_after_pattern_delete(&mut self, existing_match: WorkingMatch) {
        self.add_next_generation(existing_match.errors + 1,
                                 existing_match.starting_char,
                                 existing_match.length);
    }

    fn add_after_text_delete(&mut self,
                             existing_match: WorkingMatch,
                             child: CharLocation) {
        self.add_this_generation(existing_match.errors + 1,
                                 child,
                                 existing_match.length + 1);
    }

    /// Process a possible match/mismatch between the current
    /// pattern character and the edge leading to this child
    /// If they match, or if either is in the set of ignorable characters,
    /// then don't increment the error. Otherwise, it is a mismatch and
    /// increases error by 1.
    fn add_after_mismatch(&mut self,
                          existing_match: WorkingMatch,
                          child: CharLocation,
                          pattern_char: &char,
                          edge: &char) {
        let mut errors_after_match = existing_match.errors;
        if self.config.chars_match(edge, pattern_char) {
            // If the edge matches the character this doesn't add an error
        } else {
            // Else this is a mismatch - increment the error counter
            errors_after_match += 1;
        }
        debug!("Adding node {:?} with errors {} - match/mismatch", child, errors_after_match);
        self.add_next_generation(errors_after_match,
                                 child,
                                 existing_match.length + 1);
    }

    fn go_to_next_generation(&mut self) {
        self.matches_this_gen = self.matches_next_gen.clone();
        self.matches_next_gen = WorkingMatchesSet::empty();
    }

    fn generation_after_char_dict(&self,
                                  suffix_trie: &SuffixTrie,
                                  char_location: CharLocation) -> HashMap<char, CharLocation> {
        let this_node = suffix_trie.get_node(char_location.node_index);
        let mut result = HashMap::new();
        if char_location.index_in_edge + 1 >= this_node.edge_length {
            // This char is at the end of the string of its node, so children
            // of the char are the children of the node itself
            for (edge, child_index) in this_node.children.iter() {
                let child_location = CharLocation {
                    node_index: *child_index,
                    index_in_edge: 0,
                };
                result.insert(*edge, child_location);
            }
            debug!("Children of location {:?} are children of the node", char_location);
        } else {
            // Only one child - the charlocation after this one in the edge of
            // this node
            let new_edge_start_index = char_location.index_in_edge + 1;
            let child_location = CharLocation {
                node_index: char_location.node_index,
                index_in_edge: new_edge_start_index,
            };
            let edge = suffix_trie.str_storage[this_node.edge_start_index + new_edge_start_index].clone();
            result.insert(edge, child_location);
            debug!("Only child of location {:?} is the next character in the edge of the node", char_location);
        }
        debug!("Children are {:#?}", result);
        return result
    }

    fn find_edit_distance_ignore(&mut self,
                                 suffix_trie: &SuffixTrie,
                                 pattern: &str)
        -> Vec<Match> {
            let ascii_pattern = deunicode::deunicode(pattern);

            // Keep track of matches and how many errors they have so far
            for c in ascii_pattern.chars() {
                debug!("Matching char: {}", c);
                debug!("Matching nodes: {:#?}", self);
                while let Some(parent_match) = self.matches_this_gen.next() {
                    debug!("Parent match: {:?}", parent_match);
                    let children = self.generation_after_char_dict(suffix_trie,
                                                                   parent_match.starting_char);
                    for (edge, child) in children.iter() {
                        debug!("Considering child {}", edge);
                        self.add_after_mismatch(parent_match,
                                                *child,
                                                &c,
                                                &edge);
                        self.add_after_pattern_delete(parent_match);
                        self.add_after_text_delete(parent_match,
                                                   *child);
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
                let leaf_children = suffix_trie.get_all_leaf_descendants(parent_match.starting_char.node_index);
                debug!("Matching node: {:#?} with children {:#?}",
                       parent_match.starting_char.node_index,
                       leaf_children);
                let parent_matches = suffix_trie.match_array_from_leaves(leaf_children,
                                                                         parent_match.length,
                                                                         parent_match.errors);
                matches.extend(parent_matches);
            }
            matches.sort();
            matches
        }

    fn find_exact(&mut self, suffix_trie: &SuffixTrie, pattern: &str) -> Vec<Match> {
        let mut parent: &SubTrie = suffix_trie.get_node(0);
        let ascii_pattern = deunicode::deunicode(pattern);
        let mut string_iterator = ascii_pattern.chars();

        let mut found_mismatch = false;
        while let Some(c) = &string_iterator.next() {
            if let Some(child_index) = parent.get_child_index(*c) {
                let edge_match = suffix_trie.consume_all_shared_length(*child_index,
                                                                       &mut string_iterator,
                                                                       &self.config);
                match edge_match.overlap_type {
                    EdgeMatchKind::WholeMatch =>  {
                        // Continue iterating
                        parent = suffix_trie.get_node(*child_index);
                    },
                    EdgeMatchKind::Diverge(_) => {
                        found_mismatch = true;
                        break;
                    },
                    EdgeMatchKind::EarlyStop => {
                        // Match ended in the middle of the edge (i.e. the rest of our
                        // string is shorter than the edge, but all characters
                        // match).
                        // Set up parent node, but since've we're out of characters
                        // we shouldn't end up iterating more
                        parent = suffix_trie.get_node(*child_index);
                        assert!(!&string_iterator.next().is_some())
                    }
                }
            } else {
                // No match
                found_mismatch = true;
                break;
            }
        }

        let mut matches = Vec::new();
        if !found_mismatch {
            let leaves = suffix_trie.get_all_leaf_descendants(parent.node_index);
            info!("Found {} leaves below parent {}",
                  leaves.len(),
                  parent.node_index);
            matches = suffix_trie.match_array_from_leaves(leaves, ascii_pattern.len(), 0);
            matches.sort();
        }
        info!("Found {} matches", matches.len());
        matches
    }
}

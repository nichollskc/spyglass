use std::collections::{HashMap,HashSet};

#[cfg(test)]
mod tests {
    use super::*;

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
}

#[derive(Debug)]
struct SuffixTrie {
    // Actual trie structure
    root: SubTrie,
    // Place to store entire string - keeps ownership simple
    str_storage: String,
    // Place to store all the nodes
    node_storage: Vec<SubTrie>,
}

#[derive(Debug)]
struct SubTrie {
    // Index of this node in the overall array
    node_index: usize,
    // List of children node indices
    children: HashMap<char, usize>,
    // List of indices at which this suffix is present
    leaf_children: Vec<usize>,
}

impl SuffixTrie {
    fn new(string: &str) -> Self {
        let mut suffix_trie = SuffixTrie {
            str_storage: String::from(string.clone()) + "$0",
            node_storage: vec![SubTrie::empty(0)],
            root: SubTrie::empty(0),
        };

        for (index, _c) in string.char_indices() {
            let suffix = &string[index..];
            suffix_trie.add_string(suffix, index);
        }
        suffix_trie
    }

    fn len(&self) -> usize {
        self.node_storage.len()
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

    fn add_string(&mut self, string: &str, string_key: usize) {
        let mut parent_index = 0;

        for c in string.chars() {
            let child_index = self.add_edge(c, parent_index);
            parent_index = child_index;
        }

        let parent: &mut SubTrie = self.get_node_mut(parent_index);
        parent.add_leaf_child(string_key);
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

    fn leaf_count(&self) -> usize { unimplemented!() } //self.root.leaf_count() }
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

use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size() {
        let trie = SuffixTrie::new("banana");
        assert_eq!(trie.leaf_count(), 7);
    }
}

struct SuffixTrie {
    // Actual trie structure
    root: SubTrie,
    // Place to store entire string - keeps ownership simple
    str_storage: String,
}

struct SubTrie {
    // Label of edge from parent to this node
    // List of children nodes
    children: HashMap<char, SubTrie>,
    // List of special children nodes which correspond to the
    //   end of a suffix
    leaf_children: Vec<LeafChild>,
}

struct LeafChild {
    index: usize,
}

impl SuffixTrie {
    fn new(string: &str) -> Self {
        let mut suffix_trie = SuffixTrie {
            str_storage: String::from(string.clone()) + "$0",
            root: SubTrie::empty(),
        };

        for (index, c) in string.char_indices() {
            let suffix = &string[index..];
            suffix_trie.root.add_string(suffix);
        }
        suffix_trie
    }

    fn leaf_count(&self) -> usize { self.root.leaf_count() }
}

impl SubTrie {
    fn empty() -> Self {
        SubTrie {
            children: HashMap::new(),
            leaf_children: vec![],
        }
    }

    fn add_string(&self, string: &str) {
        let mut parent: &SubTrie = self;
        for c in string.chars() {
            let mut maybe_child: Option<&SubTrie> = parent.children.get(&c); 
            // If there was already a child node, simply unwrap
            // Otherwise create a new empty child node and add it to the parent
            let mut child = maybe_child.unwrap_or({
                let empty = SubTrie::empty();
                parent.children.insert(c, &empty);
                &empty
            });

            let mut parent = child;
        }
    }

    fn leaf_count(&self) -> usize {
        0
    }

    fn list_breadth_first(&self) -> Vec<&SubTrie> {
        let mut all_nodes: Vec<&SubTrie> = Vec::new();
        let mut to_process: Vec<&SubTrie> = self.children.values().collect();

        while to_process.len() > 0 {
            let node = to_process.remove(0);
            let child_nodes: Vec<&SubTrie> = node.children.values().collect();
            to_process.extend(child_nodes);
            all_nodes.push(node);
        }

        all_nodes
    }

}

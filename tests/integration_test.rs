use std::cmp;
use std::collections::HashMap;

use utilities;

use spyglass::{Match,SuffixTrie};

fn compare_match_indices(matches: Vec<Match>, indices: Vec<usize>) {
    let match_indices: Vec<usize> = matches.iter().map(|l| l.index_in_str).collect();
    assert_eq!(match_indices, indices);
}

fn compare_matches(mut expected: Vec<Match>, mut matches: Vec<Match>) {
    expected.sort();
    matches.sort();
    assert_eq!(expected, matches);
}

#[test]
fn serialize_tests() {
    utilities::init_testing();
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
fn find_matches() {
    utilities::init_testing();
    let trie = SuffixTrie::new("aba");
    println!("Result is {:#?}", trie);

    let matches = trie.find_exact("a", false);
    compare_match_indices(matches, vec![0, 2]);

    let trie = SuffixTrie::new("bananaBal");
    println!("Result is {:#?}", trie);

    let matches = trie.find_exact("an", false);
    compare_match_indices(matches, vec![1, 3]);

    let matches = trie.find_exact("ab", false);
    compare_match_indices(matches, vec![]);
}

#[test]
fn find_matches_0_edit_no_branches() {
    utilities::init_testing();
    let trie = SuffixTrie::new("abcdef");
    println!("Result is {:#?}", trie);

    let matches = trie.find_edit_distance("bcd", 0);
    compare_match_indices(matches, vec![1]);
}

#[test]
fn find_matches_0_edit() {
    utilities::init_testing();
    let trie = SuffixTrie::new("aba");
    println!("Result is {:#?}", trie);

    let matches = trie.find_edit_distance("a", 0);
    compare_match_indices(matches, vec![0, 2]);

    let trie = SuffixTrie::new("bananaBal");
    println!("Result is {:#?}", trie);

    let matches = trie.find_edit_distance("an", 0);
    compare_match_indices(matches, vec![1, 3]);

    let matches = trie.find_edit_distance("ab", 0);
    compare_match_indices(matches, vec![]);
}

#[test]
fn find_case_insensitive() {
    let trie = SuffixTrie::new("aBcABCabcaBcaBC");
    println!("Result is {:#?}", trie);

    let ignored = HashMap::new();
    let matches = trie.find_edit_distance_ignore("ABc", 0, ignored.clone(), false);
    compare_match_indices(matches, vec![]);

    let matches = trie.find_edit_distance_ignore("ABC", 0, ignored.clone(), true);
    compare_match_indices(matches, vec![0, 3, 6, 9, 12]);

    let matches = trie.find_edit_distance_ignore("Abc", 0, ignored.clone(), true);
    compare_match_indices(matches, vec![0, 3, 6, 9, 12]);

    let matches = trie.find_exact("ABc", false);
    compare_match_indices(matches, vec![]);

    let matches = trie.find_exact("ABC", true);
    compare_match_indices(matches, vec![0, 3, 6, 9, 12]);

    let matches = trie.find_exact("Abc", true);
    compare_match_indices(matches, vec![0, 3, 6, 9, 12]);
}

#[test]
fn find_matches_mismatch() {
    utilities::init_testing();
    let trie = SuffixTrie::new("abcXef abXdef");
    println!("Result is {:#?}", trie);

    let matches = trie.find_edit_distance("abcdef", 1);
    compare_match_indices(matches, vec![0, 7]);
}

#[test]
fn find_matches_insert_delete() {
    utilities::init_testing();
    let trie = SuffixTrie::new("abcXdefg");
    println!("Result is {:#?}", trie);

    // Delete from text
    let matches = trie.find_edit_distance("abcdefg", 1);
    compare_match_indices(matches, vec![0]);

    // Delete from pattern
    let matches = trie.find_edit_distance("aXbc", 1);
    compare_match_indices(matches, vec![0]);
}

#[test]
fn find_partial_matches_ignore() {
    utilities::init_testing();
    let trie = SuffixTrie::new("He wracked wrack'd wrack'ed");
    println!("Result is {:#?}", trie);

    let mut ignored = HashMap::new();
    ignored.insert('e', true);
    ignored.insert('\'', true);
    let matches = trie.find_edit_distance_ignore("wrackd", 0, ignored.clone(), false);
    compare_match_indices(matches, vec![3, 11, 19]);
    let matches = trie.find_edit_distance_ignore("wrack'de", 0, ignored.clone(), false);
    compare_match_indices(matches, vec![3, 11, 19]);
}

#[test]
fn matches_from_directory() {
    utilities::init_testing();
    let trie = SuffixTrie::from_directory("./resources/tests/simple/").unwrap();
    println!("Result is {:#?}", trie);

    let matches_a = trie.find_exact("ABCDEF", false);
    let matches_e = trie.find_exact("EFGHIJ", false);
    let matches_e_error = trie.find_edit_distance("EFxHIJ", 1);
    let matches_e_del = trie.find_edit_distance("EFHIJ", 1);
    let matches_e_ins = trie.find_edit_distance("EFGxHIJ", 1);
    let matches_h = trie.find_exact("HIJ\nA", false);

    let mut expected_a: Vec<Match> = vec![];
    let mut expected_e: Vec<Match> = vec![];
    let mut expected_e_error: Vec<Match> = vec![];
    let mut expected_e_del: Vec<Match> = vec![];
    let mut expected_e_ins: Vec<Match> = vec![];
    let mut expected_h: Vec<Match> = vec![];
    for text_index in vec![0, 1, 2] {
        for line in 0..7 {
            let first_match_a = Match {
                text_index,
                index_in_str: 0 + 22*line,
                start_line: line,
                end_line: line,
                length: 6,
                errors: 0,
            };
            let second_match_a = Match {
                index_in_str: 11 + 22*line,
                ..first_match_a
            };

            let first_match_e = Match {
                index_in_str: 4 + 22*line,
                ..first_match_a
            };
            let second_match_e = Match {
                index_in_str: 15 + 22*line,
                ..first_match_a
            };
            let first_match_e_error = Match {
                errors: 1,
                ..first_match_e
            };
            let second_match_e_error = Match {
                errors: 1,
                ..second_match_e
            };
            expected_a.push(first_match_a);
            expected_a.push(second_match_a);
            expected_e.push(first_match_e);
            expected_e.push(second_match_e);
            expected_e_error.push(first_match_e_error.clone());
            expected_e_error.push(second_match_e_error.clone());
            expected_e_del.push(first_match_e_error.clone());
            expected_e_del.push(second_match_e_error.clone());
            expected_e_ins.push(first_match_e_error.clone());
            expected_e_ins.push(second_match_e_error.clone());
        }
    }
    compare_matches(expected_a, matches_a);
    compare_matches(expected_e, matches_e);
    compare_matches(expected_e_error, matches_e_error);
    compare_matches(expected_e_del, matches_e_del);
    compare_matches(expected_e_ins, matches_e_ins);

    for text_index in vec![0, 1, 2] {
        for line in vec![0, 1, 2, 4, 5] {
            let match_h = Match {
                text_index,
                index_in_str: 18 + 22*line,
                start_line: line,
                end_line: line + 1,
                length: 5,
                errors: 0,
            };
            expected_h.push(match_h);
        }
    }
    compare_matches(expected_h, matches_h);
}

#[test]
fn match_str_is_match() {
    utilities::init_testing();
    let trie = SuffixTrie::from_directory("./resources/tests/large_100/").unwrap();
    println!("Made trie!");
    let matches = trie.find_exact("ell", false);
    for match_obj in matches {
        for context in vec![0, 1, 5, 10, 18] {
            println!("Asking for {} lines around match {:#?}", context, match_obj);
            let (before, matching, after) = trie.get_strings_of_match(&match_obj,
                                                                      context);
            assert_eq!("ell", matching);

            let line_breaks_before = if match_obj.text_index == 1 && match_obj.start_line == 18 {
                cmp::min(context, 17)
            } else {
                context
            };
            assert_eq!(before.match_indices('\n').count(), line_breaks_before);

            let line_breaks_after = if match_obj.text_index == 1 && match_obj.start_line == 93 {
                cmp::min(context + 1, 7)
            } else if match_obj.text_index == 1 && match_obj.start_line == 98 {
                cmp::min(context + 1, 2)
            } else {
                context + 1
            };
            assert_eq!(after.match_indices('\n').count(), line_breaks_after);
        }
    }
}

#[test]
fn build_trie_from_file() {
    utilities::init_testing();
    let trie = SuffixTrie::from_file("resources/tests/simple/small.txt").unwrap();
    println!("Result is {:#?}", trie);
}

#[test]
fn find_exact() {
    let trie = SuffixTrie::from_file("resources/tests/simple/small.txt").unwrap();
    println!("Result is {:#?}", trie);
    let matches = trie.find_exact("drunken", false);
    assert_eq!(matches.len(), 3);
    let matches = trie.find_exact("early", false);
    assert_eq!(matches.len(), 1);
}

#[test]
fn build_dodgy_characters() {
    let _trie = SuffixTrie::new("father’s");
    let _trie = SuffixTrie::new("Ælfred");
    let _trie = SuffixTrie::new("…he");
    let _trie = SuffixTrie::new("father’s xxÆlfredxxÆlfredxxAlfredxxAElfred…he");
    let _trie = SuffixTrie::new("father’s xxÆlfredxxÆlfredxxAlfredxxAElfred<<STOP>>…he");
}

#[test]
fn match_dodgy_characters() {
    utilities::init_testing();
    //                          012345678901234567890123456789012345678901
    let trie = SuffixTrie::new("father’s xxÆlfredxxÆlfredxxAlfrixxAElfredxx<<STOP>>…he");
    println!("{:#?}", trie);
    let alf_matches = trie.find_exact("xxÆlf", false);
    let alf_matches_edit_0 = trie.find_edit_distance("xxÆlf", 0);
    let alf_match = Match {
        text_index: 0,
        index_in_str: 9,
        length: 6,
        start_line: 0,
        end_line: 0,
        errors: 0,
    };
    let alf_match2 = Match {
        index_in_str: 18,
        ..alf_match
    };
    let alf_match4 = Match {
        index_in_str: 34,
        ..alf_match
    };
    let alf_expected = vec![alf_match.clone(), alf_match2.clone(), alf_match4.clone()];
    compare_matches(alf_expected.clone(), alf_matches);
    compare_matches(alf_expected, alf_matches_edit_0);

    let alfric_matches = trie.find_edit_distance("xxÆlfricxx", 2);
    let alfric_match = Match {
        errors: 2,
        length: 11,
        ..alf_match
    };
    let alfric_match2 = Match {
        index_in_str: 18,
        ..alfric_match
    };
    let alfric_match3 = Match {
        index_in_str: 27,
        length: 9,
        ..alfric_match
    };
    let alfric_match4 = Match {
        index_in_str: 34,
        ..alfric_match
    };
    let alfric_expected = vec![alfric_match, alfric_match2, alfric_match3, alfric_match4];
    compare_matches(alfric_expected, alfric_matches);
}

# Spyglass

Tool to search through texts using a Suffix Trie built up from sentences of the text.

# Search types

1. Single wildcard

`te?t` matches `test` and `text`

2. Multi character wildcard

`mush*` matches `mushroom` and `mushy` and `mush`

Equivalent to `\w*` in regex

3. Multi word wildcard

`this ** rabbit` matches `this rabbit` and `this enormous rabbit` and `this big furry rabbit`

4. Approximate match using edit distance

`he repl'd` with edit distance 2 matches `he replied`

5. Searching with list of ignorable characters

E.g. ignoring vowels and punctuation `wracked` matches `rack'd` and `wrecked`

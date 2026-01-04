# treeq

A code rewriting tool using JQ filters and Tree-Sitter syntax trees. It's built on top of the
awesome [jaq](https://github.com/01mf02/jaq), a Rust implementation of the JQ language.

> [!NOTE]
> This project is currently very much just an experiment that I hacked together in a few hours.
> Among other things it lacks proper error handling and support for nested replacing.

## Concept

I work on a repo that contains a lot of JSON files that occasionally need to be changed. I have
found jq to be a very useful tool for this purpose and I've wished that changing source code in
other languages was comparably easy. After thinking about this problem for a while I came up with
the following concept.

While turning a Tree-sitter syntax tree into a JSON-like data structure for manipulating is
relatively easy, the problem is that then we'd need a way to turn the edited syntax tree back into
text. While generating code is generally a much easier problem than parsing it, Tree-sitter does
not support printing edited trees. But then I realized that instead of editing the syntax tree we
could simply leave notes in the tree to replace specific nodes with a string.

## Commands

### inspect

Usage: `treeq <LANG> inspect <FILTER> <PATH>`

With `treeq inspect` you can inspect the syntax tree of a single source file 

#### Example

Finding all the string literals in a file:

```sh
treeq rust inspect '[recurse | select(.kind? == "string_content") | .value]' src/main.rs
```

<details>
  <summary>Output</summary>

  ```json
  [
    "kind",
    "_treeq_replace",
    "kind",
    "start_byte",
    "end_byte",
    "children",
    "value",
    "./defs.jq",
    "{path}:",
    "{block}"
  ]
  ```
</details>

### find

Usage: `treeq <LANG> find <FILTER> <PATH>`

#### Example

Finding all macro invocations in the `src` folder.

```sh
treeq rust find 'walk(
   if .kind? == "macro_invocation" ?// false then
      highlight("Look at this glorious macro!")
   else . end
)' src
```

<details>
  <summary>Output</summary>

  ```
      ╭─[src/main.rs]
      │
   89 │       thread_local! {
      ┆       ▲
      ┆ ╭─────╯
   90 │ │         pub static KIND: Rc<String> = Rc::new("kind".to_string());
      ┆ │
   91 │ │         pub static START_BYTE: Rc<String> = Rc::new("start_byte".to_string());
      ┆ │
   92 │ │         pub static END_BYTE: Rc<String> = Rc::new("end_byte".to_string());
      ┆ │
   93 │ │         pub static CHILDREN: Rc<String> = Rc::new("children".to_string());
      ┆ │
   94 │ │         pub static VALUE: Rc<String> = Rc::new("value".to_string());
      ┆ │
   95 │ │     }
      ┆ │     ▲
      ┆ │     │
      ┆ ╰─────┴─ Look at this glorious macro!

   ───╯
       ╭─[src/main.rs]
       │
   183 │     let defs = jaq_core::load::parse(include_str!("./defs.jq"), |p| p.defs())
       ┆                                      ────────────┬────────────
       ┆                                                  │
       ┆                                                  ╰──────────────────────────── Look at this glorious macro!

   ────╯
       ╭─[src/main.rs]
       │
   210 │     assert!(out.next().is_none());
       ┆     ──────────────┬──────────────
       ┆                   │
       ┆                   ╰──────────────── Look at this glorious macro!

   ────╯
       ╭─[src/main.rs]
       │
   282 │                         ResultNode::Highlight { .. } => todo!(),
       ┆                                                         ───┬───
       ┆                                                            │
       ┆                                                            ╰───── Look at this glorious macro!

   ────╯
       ╭─[src/main.rs]
       │
   283 │                         ResultNode::Replace { .. } => todo!(),
       ┆                                                       ───┬───
       ┆                                                          │
       ┆                                                          ╰───── Look at this glorious macro!

   ────╯
       ╭─[src/main.rs]
       │
   342 │               println!(
       ┆               ▲
       ┆ ╭─────────────╯
   343 │ │                 "{}[{path}]\n{block}\n{}",
       ┆ │
   344 │ │                 block.prologue(),
       ┆ │
   345 │ │                 block.epilogue()
       ┆ │
   346 │ │             );
       ┆ │             ▲
       ┆ │             │
       ┆ ╰─────────────┴── Look at this glorious macro!

   ────╯
       ╭─[src/main.rs]
       │
   364 │               println!(
       ┆               ▲
       ┆ ╭─────────────╯
   365 │ │                 "{}[{path}]\n{block}\n{}",
       ┆ │
   366 │ │                 block.prologue(),
       ┆ │
   367 │ │                 block.epilogue()
       ┆ │
   368 │ │             );
       ┆ │             ▲
       ┆ │             │
       ┆ ╰─────────────┴── Look at this glorious macro!

   ────╯
   ```
</details>

### replace

Usage: `treeq <LANG> replace <FILTER> <PATH>`

#### Example

Making every struct public.

```sh
treeq rust replace 'walk(
   if
      (.kind? == "struct_item" ?// false)
      and (.children? ?// [] | any(.kind? == "visibility_modifier") | not)
   then
      replace(["pub ", .])
   else . end
)' src
```

## Supported Languages

Currently only a few languages are supported:

- JavaScript
- Markdown (This one is currently only partially usable as the grammar seems to have a lot of text
  content in unnamed nodes, which get omitted in the tree we expose.)
- PHP
- Rust
- TypeScript

New languages can be added in `src/langs.rs` as long as they have a Tree-sitter grammar available as
a crate.

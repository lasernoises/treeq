mod langs;

use std::{collections::HashMap, path::PathBuf, rc::Rc};

use clap::{Parser as _, Subcommand};
use codesnake::{Block, CodeWidth, Label, LineIndex};
use ignore::{WalkBuilder, types::TypesBuilder};
use indexmap::IndexMap;
use jaq_core::{
    Compiler, Ctx, RcIter,
    load::{Arena, Loader},
};
use jaq_json::Val;
use serde::Deserialize;
use serde_json::Value;
use tree_sitter::{Node, Parser, TreeCursor};

use crate::langs::CliLang;

#[derive(clap::Parser)]
struct Cli {
    lang: CliLang,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Inspect { filter: String, path: PathBuf },
    InspectArg { filter: String, code: String },
    Find { filter: String, path: PathBuf },
    Replace { filter: String, path: PathBuf },
}

#[derive(Deserialize, Debug)]
#[serde(tag = "kind")]
enum ResultNode {
    #[serde(rename = "_treeq_replace")]
    Replace {
        start_byte: usize,
        end_byte: usize,
        entries: Vec<ReplaceEntry>,
    },

    #[serde(rename = "_treeq_highlight")]
    Highlight {
        start_byte: usize,
        end_byte: usize,
        message: String,
    },

    #[serde(untagged)]
    #[allow(unused)]
    TreeSitter {
        kind: String,
        start_byte: usize,
        end_byte: usize,
        children: Option<Vec<ResultNode>>,
        value: Option<Box<NodeValue>>,
        #[serde(flatten)]
        extra: HashMap<String, ResultNode>,
    },
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum NodeValue {
    #[allow(unused)]
    String(String),
    Node(ResultNode),
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum ReplaceEntry {
    String(String),
    Node(ResultNode),
}

struct Lang {
    // TODO: Change this to a list of extensions. The types from ignore are sometimes not exactly
    // what we want.
    /// File type from the ignore crate. See
    /// <https://github.com/BurntSushi/ripgrep/blob/master/crates/ignore/src/default_types.rs>.
    file_type: &'static str,
    language_fn: tree_sitter_language::LanguageFn,
}

mod keys {
    use std::rc::Rc;

    thread_local! {
        pub static KIND: Rc<String> = Rc::new("kind".to_string());
        pub static START_BYTE: Rc<String> = Rc::new("start_byte".to_string());
        pub static END_BYTE: Rc<String> = Rc::new("end_byte".to_string());
        pub static CHILDREN: Rc<String> = Rc::new("children".to_string());
        pub static VALUE: Rc<String> = Rc::new("value".to_string());
    }
}

fn main() {
    let cli = Cli::parse();

    let lang = cli.lang.to_lang();

    let mut parser = Parser::new();
    parser.set_language(&lang.language_fn.into()).unwrap();

    match &cli.command {
        Command::Inspect { filter, path } => {
            let source = std::fs::read_to_string(path).unwrap();
            let value = eval(&mut parser, filter, &source);

            serde_json::to_writer_pretty(std::io::stdout(), &value).unwrap();
        }
        Command::InspectArg { filter, code } => {
            let value = eval(&mut parser, filter, &code);

            serde_json::to_writer_pretty(std::io::stdout(), &value).unwrap();
        }
        Command::Find { filter, path } => {
            for entry in WalkBuilder::new(path)
                .types(
                    TypesBuilder::new()
                        .add_defaults()
                        .select(lang.file_type)
                        .build()
                        .unwrap(),
                )
                .build()
            {
                let entry = entry.unwrap();

                if !entry.file_type().map_or(false, |t| t.is_file()) {
                    continue;
                }

                let source = std::fs::read_to_string(entry.path()).unwrap();
                let value = eval(&mut parser, filter, &source);

                let result: ResultNode = serde_json::from_value(value).unwrap();

                let line_index = LineIndex::new(&source);

                print(&result, entry.path().to_str().unwrap(), &line_index);
            }
        }
        Command::Replace { filter, path } => {
            for entry in WalkBuilder::new(path)
                .types(
                    TypesBuilder::new()
                        .add_defaults()
                        .select(lang.file_type)
                        .build()
                        .unwrap(),
                )
                .build()
            {
                let entry = entry.unwrap();

                if !entry.file_type().map_or(false, |t| t.is_file()) {
                    continue;
                }

                let source = std::fs::read_to_string(entry.path()).unwrap();
                let value = eval(&mut parser, filter, &source);

                let result: ResultNode = serde_json::from_value(value).unwrap();

                let mut adjustment = 0;
                let mut modified = source.clone();

                replace(&result, &source, &mut modified, &mut adjustment);

                std::fs::write(entry.path(), &modified).unwrap();
            }
        }
    }
}

fn eval(parser: &mut Parser, filter: &str, input: &str) -> Value {
    let tree = parser.parse(&input, None).unwrap();

    let json = node_to_json(&tree.root_node(), &mut tree.walk(), &input);

    let defs = jaq_core::load::parse(include_str!("./defs.jq"), |p| p.defs())
        .unwrap()
        .into_iter();

    let loader = Loader::new(jaq_std::defs().chain(jaq_json::defs()).chain(defs));
    let arena = Arena::default();

    let modules = loader
        .load(
            &arena,
            jaq_core::load::File {
                code: &filter,
                path: (),
            },
        )
        .unwrap();

    let filter = Compiler::default()
        .with_funs(jaq_std::funs().chain(jaq_json::funs()))
        .compile(modules)
        .unwrap();

    let inputs = RcIter::new(std::iter::empty());

    let mut out = filter.run((Ctx::new([], &inputs), json));

    let result = out.next().unwrap().unwrap();
    assert!(out.next().is_none());

    result.into()
}

fn node_to_json<'tree>(node: &Node<'tree>, cursor: &mut TreeCursor<'tree>, code: &str) -> Val {
    let mut map = IndexMap::with_capacity_and_hasher(8, foldhash::fast::RandomState::default());

    map.insert(
        keys::KIND.with(|k| k.clone()),
        node.kind().to_string().into(),
    );

    map.insert(
        keys::START_BYTE.with(|k| k.clone()),
        (node.start_byte() as isize).into(),
    );
    map.insert(
        keys::END_BYTE.with(|k| k.clone()),
        (node.end_byte() as isize).into(),
    );

    let children: Vec<Val> = node
        .named_children(cursor)
        .collect::<Vec<_>>()
        .iter()
        .enumerate()
        .filter_map(|(i, child)| {
            if let Some(name) = node.field_name_for_named_child(i as u32) {
                let value = node_to_json(child, cursor, code);

                map.insert(name.to_string().into(), value);

                None
            } else {
                Some(node_to_json(&child, cursor, code))
            }
        })
        .collect();

    if !children.is_empty() {
        map.insert(
            keys::CHILDREN.with(|k| k.clone()),
            Val::Arr(Rc::new(children)),
        );
    }

    if node.named_child_count() == 0 {
        map.insert(
            keys::VALUE.with(|k| k.clone()),
            code[node.start_byte()..node.end_byte()].to_string().into(),
        );
    }

    Val::obj(map)
}

fn replace(node: &ResultNode, source: &str, modified: &mut String, adjustment: &mut isize) {
    match node {
        ResultNode::Highlight { .. } => (),
        ResultNode::Replace {
            start_byte,
            end_byte,
            entries,
        } => {
            // TODO: reuse this buffer
            let mut tmp = String::new();

            for entry in entries {
                match entry {
                    ReplaceEntry::String(string) => tmp.push_str(string),
                    ReplaceEntry::Node(result_node) => match result_node {
                        ResultNode::Highlight { .. } => todo!(),
                        ResultNode::Replace { .. } => todo!(),
                        &ResultNode::TreeSitter {
                            start_byte,
                            end_byte,
                            ..
                        } => tmp.push_str(&source[start_byte..end_byte]),
                    },
                }
            }

            modified.replace_range(
                start_byte.checked_add_signed(*adjustment).unwrap()
                    ..end_byte.checked_add_signed(*adjustment).unwrap(),
                &tmp,
            );

            *adjustment += tmp
                .len()
                .checked_signed_diff(end_byte - start_byte)
                .unwrap();
        }
        ResultNode::TreeSitter {
            children,
            extra,
            value,
            ..
        } => {
            for child in children.iter().flatten() {
                replace(child, source, modified, adjustment);
            }

            if let Some(NodeValue::Node(child)) = value.as_deref() {
                replace(child, source, modified, adjustment);
            }

            for (_, child) in extra {
                replace(child, source, modified, adjustment);
            }
        }
    }
}

fn print(node: &ResultNode, path: &str, line_index: &LineIndex) {
    match node {
        &ResultNode::Highlight {
            start_byte,
            end_byte,
            ref message,
        } => {
            let block = Block::new(
                line_index,
                std::iter::once(
                    Label::new(start_byte..end_byte).with_text(CodeWidth::new(message, 0)),
                ),
            )
            .unwrap();

            let block = block.map_code(|c| CodeWidth::new(c, c.len()));

            println!(
                "{}[{path}]\n{block}\n{}",
                block.prologue(),
                block.epilogue()
            );
        }
        &ResultNode::Replace {
            start_byte,
            end_byte,
            entries: _,
        } => {
            let block = Block::new(
                line_index,
                std::iter::once(
                    Label::new(start_byte..end_byte)
                        .with_text(CodeWidth::new("This will be replaced.", 0)),
                ),
            )
            .unwrap();

            let block = block.map_code(|c| CodeWidth::new(c, c.len()));

            println!(
                "{}[{path}]\n{block}\n{}",
                block.prologue(),
                block.epilogue()
            );
        }
        ResultNode::TreeSitter {
            children,
            extra,
            value,
            ..
        } => {
            for child in children.iter().flatten() {
                print(child, path, line_index);
            }

            if let Some(NodeValue::Node(child)) = value.as_deref() {
                print(child, path, line_index);
            }

            for (_, child) in extra {
                print(child, path, line_index);
            }
        }
    }
}

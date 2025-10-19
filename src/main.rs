use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
};

use clap::{Parser as _, Subcommand};
use indexmap::IndexMap;
use jaq_core::{
    Compiler, Ctx, RcIter,
    load::{Arena, Loader},
};
use jaq_json::Val;
use serde::Deserialize;
use serde_json::Value;
use tree_sitter::{Node, Parser, TreeCursor};

#[derive(clap::Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Inspect { filter: String, path: PathBuf },
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

    #[serde(untagged)]
    TreeSitter {
        kind: String,
        start_byte: usize,
        end_byte: usize,
        children: Option<Vec<ResultNode>>,
        value: Option<String>,
        #[serde(flatten)]
        extra: HashMap<String, ResultNode>,
    },
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum ReplaceEntry {
    String(String),
    Node(ResultNode),
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Command::Inspect { filter, path } => {
            let value = eval(filter, path);

            serde_json::to_writer_pretty(std::io::stdout(), &value).unwrap();
        }
        Command::Find { filter, path } => todo!(),
        Command::Replace { filter, path } => {
            let value = eval(filter, path);

            let result: ResultNode = serde_json::from_value(value).unwrap();
            dbg!(result);
        }
    }
}

fn eval(filter: &str, path: &Path) -> Value {
    let input = std::fs::read_to_string(path).unwrap();

    let mut parser = Parser::new();

    parser
        .set_language(&tree_sitter_php::LANGUAGE_PHP.into())
        .unwrap();

    let tree = parser.parse(&input, None).unwrap();

    let json = node_to_json(&tree.root_node(), &mut tree.walk(), &input);

    let loader = Loader::new(jaq_std::defs().chain(jaq_json::defs()));
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

    map.insert("kind".to_string().into(), node.kind().to_string().into());

    map.insert(
        "start_byte".to_string().into(),
        (node.start_byte() as isize).into(),
    );
    map.insert(
        "end_byte".to_string().into(),
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
        map.insert("children".to_string().into(), Val::Arr(Rc::new(children)));
    }

    if node.child_count() == 0 {
        map.insert(
            "value".to_string().into(),
            code[node.start_byte()..node.end_byte()].to_string().into(),
        );
    }

    Val::obj(map)
}

use std::{path::PathBuf, rc::Rc};

use clap::Parser as _;
use indexmap::IndexMap;
use jaq_core::{
    Compiler, Ctx, RcIter,
    load::{Arena, Loader},
};
use jaq_json::Val;
use tree_sitter::{Node, Parser, TreeCursor};

#[derive(clap::Parser)]
struct Cli {
    filter: String,
    path: PathBuf,
}

fn main() {
    let cli = Cli::parse();

    let mut parser = Parser::new();

    let input = std::fs::read_to_string(cli.path).unwrap();

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
                code: &cli.filter,
                path: (),
            },
        )
        .unwrap();

    let filter = Compiler::default()
        .with_funs(jaq_std::funs().chain(jaq_json::funs()))
        .compile(modules)
        .unwrap();

    let inputs = RcIter::new(std::iter::empty());

    let out = filter.run((Ctx::new([], &inputs), json));

    for out in out {
        let out = out.unwrap();

        let value: serde_json::Value = serde_json::Value::from(out);

        serde_json::to_writer_pretty(std::io::stdout(), &value).unwrap();
    }
}

fn node_to_json<'tree>(node: &Node<'tree>, cursor: &mut TreeCursor<'tree>, code: &str) -> Val {
    let mut map = IndexMap::with_capacity_and_hasher(8, foldhash::fast::RandomState::default());

    map.insert("kind".to_string().into(), node.kind().to_string().into());

    let children: Vec<Val> = node
        .named_children(cursor)
        .collect::<Vec<_>>()
        .iter()
        .enumerate()
        .filter_map(|(i, child)| {
            if let Some(name) = node.field_name_for_named_child(i as u32) {
                let value;
                if child.child_count() == 0 {
                    value = code[child.start_byte()..child.end_byte()]
                        .to_string()
                        .into();
                } else {
                    value = node_to_json(child, cursor, code);
                }

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

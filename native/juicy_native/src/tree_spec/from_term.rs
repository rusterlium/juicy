use std::collections::HashMap;
use ::rustler::{ NifTerm, NifResult, NifError };
use ::rustler::types::list::NifListIterator;
use ::rustler::types::map::NifMapIterator;
use ::rustler::types::atom::NifAtom;

use super::{
    NodeOptions,
    NodeId,
    Node,
    NodeVariant,
    Spec,
};

mod atoms {
    rustler_atoms! {
        atom stream;
        atom any;
        atom map;
        atom map_keys;
        atom array;
        atom struct_atom;
        atom atom_mappings;
        atom ignore_not_mapped;
    }
}

fn read_opts<'a>(term: NifTerm<'a>, stream_collect: bool) -> NifResult<NodeOptions> {
    let iterator: NifListIterator = term.decode()?;
    let mut opts = NodeOptions::default();
    for decoded in iterator.map(|term| term.decode::<(NifTerm, NifTerm)>()) {
        let (key, value) = decoded?;

        if atoms::stream() == key {
            opts.stream = value.decode()?;
        } else if atoms::struct_atom() == key {
            opts.struct_atom = Some(value.decode()?);
        } else if atoms::atom_mappings() == key {
            let mut map: HashMap<String, NifAtom> = HashMap::new();
            let iterator: NifMapIterator = value.decode()?;
            for (key_term, value_term) in iterator {
                let key: String = key_term.decode()?;
                let value: NifAtom = value_term.decode()?;
                map.insert(key, value);
            }
            opts.atom_mappings = Some(map);
        } else if atoms::ignore_not_mapped() == key {
            opts.ignore_not_mapped = value.decode()?;
        }

    }
    opts.stream_collect = opts.stream | stream_collect;
    Ok(opts)
}

fn read_node<'a>(node: NifTerm<'a>, nodes: &mut Vec<Node>, parent: NodeId, stream_collect: bool) -> NifResult<NodeId> {
    let current = NodeId(nodes.len());

    // Arity 3
    match node.decode::<(NifTerm, NifTerm, NifTerm)>() {
        Ok((typ, opts, data)) => {
            let opts = read_opts(opts, stream_collect)?;
            let child_stream_collect = opts.stream_collect;

            return if atoms::map() == typ {
                nodes.push(Node {
                    variant: NodeVariant::Sentinel,
                    options: opts,
                    parent: Some(parent),
                });

                let child = read_node(data, nodes, current, child_stream_collect)?;
                nodes[current.0].variant = NodeVariant::Map {
                    child: child,
                };

                Ok(current)
            } else if atoms::map_keys() == typ {
                nodes.push(Node {
                    variant: NodeVariant::Sentinel,
                    options: opts,
                    parent: Some(parent),
                });

                let mut children = HashMap::<String, NodeId>::new();
                for (key, value) in data.decode::<NifMapIterator>()? {
                    let child = read_node(value, nodes, current, child_stream_collect)?;
                    children.insert(key.decode()?, child);
                }
                nodes[current.0].variant = NodeVariant::MapKeys {
                    children: children,
                };

                Ok(current)
            } else if atoms::array() == typ {
                nodes.push(Node {
                    variant: NodeVariant::Sentinel,
                    options: opts,
                    parent: Some(parent),
                });

                let child = read_node(data, nodes, current, child_stream_collect)?;
                nodes[current.0].variant = NodeVariant::Array {
                    child: child,
                };

                Ok(current)
            } else {
                Err(NifError::BadArg)
            };

        },
        Err(_) => (),
    }

    // Arity 2
    match node.decode::<(NifTerm, NifTerm)>() {
        Ok((typ, opts)) => {
            let opts = read_opts(opts, stream_collect)?;

            return if atoms::any() == typ {
                nodes.push(Node {
                    variant: NodeVariant::Any,
                    options: opts,
                    parent: Some(parent),
                });
                Ok(current)
            } else {
                Err(NifError::BadArg)
            };

        },
        Err(_) => (),
    }

    Err(NifError::BadArg)
}

pub fn spec_from_term<'a>(root: NifTerm<'a>) -> NifResult<Spec> {
    let mut nodes = Vec::<Node>::new();

    let sentinel = Node {
        variant: NodeVariant::Sentinel,
        options: NodeOptions::default(),
        parent: None,
    };
    nodes.push(sentinel);
    let sentinel_id = NodeId(0);

    assert_eq!(read_node(root, &mut nodes, sentinel_id, false)?, NodeId(1));

    Ok(Spec {
        nodes: nodes,
        root: sentinel_id,
    })
}

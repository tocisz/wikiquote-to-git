use crate::text_extractor::TextExtractor;
use bimap::BiMap;
use parse_wiki_text::{DefinitionListItem, ListItem, Node, Output};
use regex::{Regex, RegexBuilder};
use std::collections::HashMap;
use std::fmt::Debug;
use serde::export::Formatter;

type Nd = usize;
type Ed = (Nd, Nd);

#[derive(Default, Debug)]
pub struct Graph {
    node_data: Vec<NodeData>,
    node_labels: BiMap<Nd, String>,
    edge_labels: HashMap<Ed, String>,
}

#[derive(Default, Debug)]
struct NodeData {
    outgoing: Vec<usize>,
    incoming: Vec<usize>,
}

impl Graph {
    pub fn add_vertex(&mut self, label: String) -> Nd {
        let new_idx = self.node_data.len();
        self.node_data.push(NodeData::default());
        self.node_labels.insert(new_idx, label);
        new_idx
    }

    pub fn add_edge(&mut self, e: Ed, label: String) {
        let (l, r) = e;
        if l < self.node_labels.len() && r < self.node_labels.len() {
            self.node_data[l].outgoing.push(r);
            self.node_data[r].incoming.push(l);
            self.edge_labels.insert(e, label);
        }
    }

    pub fn add(&mut self, vtx1: String, edge: String, vtx2: String) {
        let v1 = self.find_or_add_vertex(vtx1);
        let v2 = self.find_or_add_vertex(vtx2);
        self.add_edge((v1, v2), edge);
    }

    pub fn find_vertex(&self, label: &String) -> Option<Nd> {
        self.node_labels.get_by_right(label).map(|x| *x)
    }

    pub fn find_or_add_vertex(&mut self, label: String) -> Nd {
        if let Some(n) = self.find_vertex(&label) {
            n
        } else {
            self.add_vertex(label)
        }
    }

    pub fn roots(&self) -> Vec<Nd> {
        let mut result = Vec::new();
        for (i, n) in self.node_data.iter().enumerate() {
            if n.incoming.is_empty() {
                result.push(i);
            }
        }
        result
    }

    pub fn get_vertex_label(&self, id: Nd) -> &str {
        self.node_labels.get_by_left(&id).unwrap()
    }
}

#[derive(Default, Debug)]
pub struct CategoryExtractor {
    site: String,
    pub graph: Graph,
    pub normalizer: Normalizer
}

impl CategoryExtractor {
    pub fn set_site(&mut self, site: String) {
        self.site = site
    }

    pub fn extract(&mut self, parsed: &Output) {
        for n in &parsed.nodes {
            self.extract_node(n)
        }
    }

    pub fn extract_node(&mut self, node: &Node) {
        match node {
            Node::Category {
                target, ordinal, ..
            } => {
                let target_name = self.normalizer.normalize_category_name(*target);
                // println!("TARGET: {}", target_name);
                let mut extr = TextExtractor::new();
                extr.extract_nodes_text(ordinal);
                let label = extr.result().trim().to_string();
                // println!("ORD: {}", &label);
                self.graph.add(target_name, label, self.site.clone())
            }
            Node::DefinitionList { items, .. } => {
                for item in items {
                    self.extract_definition_item(item);
                }
            }
            Node::ExternalLink { nodes, .. } => {
                self.extract_nodes(nodes);
            }
            Node::Heading { nodes, .. } => self.extract_nodes(nodes),
            Node::Link { text, .. } => {
                self.extract_nodes(text);
            }
            Node::OrderedList { items, .. } => {
                for item in items {
                    self.extract_list_item(item);
                }
            }
            Node::Preformatted { nodes, .. } => {
                self.extract_nodes(nodes);
            }

            // should we handle somehow?
            //Node::Redirect { .. } => {},

            // TODO
            Node::Table { .. } => {}

            Node::Tag { nodes, .. } => {
                self.extract_nodes(nodes);
            }

            // should we handle somehow?
            // Node::Template { .. } => {},
            Node::UnorderedList { items, .. } => {
                for item in items {
                    self.extract_list_item(item);
                }
            }

            _ => {}
        }
    }

    pub fn extract_nodes(&mut self, nodes: &Vec<Node>) {
        for node in nodes {
            self.extract_node(node);
        }
    }

    pub fn extract_definition_item(&mut self, item: &DefinitionListItem) {
        for n in &item.nodes {
            self.extract_node(n)
        }
    }

    pub fn extract_list_item(&mut self, item: &ListItem) {
        for n in &item.nodes {
            self.extract_node(n)
        }
    }
}

pub struct Normalizer {
    kat_match: Regex,
    space_match: Regex,
    bad_chars: Vec<&'static str>,
}

impl Default for Normalizer {
    fn default() -> Self {
        let left_to_right = "\u{200E}";
        Self {
            kat_match: RegexBuilder::new(r"^Kategoria:")
                .case_insensitive(true)
                .build()
                .unwrap(),

            space_match: Regex::new(r"\s+").unwrap(),

            bad_chars: vec!(left_to_right),
        }
    }
}

impl Normalizer {
    pub fn normalize_category_name(&self, s: &str) -> String {
        let mut s = s;
        if self.kat_match.is_match(s) {
            s = &s[10..];
        } else {
            s = s;
        }
        s = s.trim();
        let s = self.space_match.replace_all(s, " ");
        let mut s = s.to_string();
        for ch in &self.bad_chars {
            s = s.replace(*ch, "");
        }
        s
    }
}

// Don't display it
impl Debug for Normalizer {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        Result::Ok(())
    }
}

impl<'a> dot::Labeller<'a, Nd, Ed> for Graph {
    fn graph_id(&'a self) -> dot::Id<'a> {
        dot::Id::new("categories").unwrap()
    }
    fn node_id(&'a self, n: &Nd) -> dot::Id<'a> {
        dot::Id::new(format!("N{}", n)).unwrap()
    }
    fn node_label(&self, n: &Nd) -> dot::LabelText {
        dot::LabelText::LabelStr(self.node_labels.get_by_left(n).unwrap().into())
    }
    fn edge_label(&self, e: &Ed) -> dot::LabelText {
        dot::LabelText::LabelStr(self.edge_labels.get(e).unwrap().into())
    }
}

impl<'a> dot::GraphWalk<'a, Nd, Ed> for Graph {
    fn nodes(&self) -> dot::Nodes<'a, Nd> {
        (0..self.node_data.len()).collect()
    }
    fn edges(&'a self) -> dot::Edges<'a, Ed> {
        let mut edges: Vec<Ed> = Vec::new();
        for (n, data) in self.node_data.iter().enumerate() {
            for m in &data.outgoing {
                edges.push((n, *m));
            }
        }
        dot::Edges::from(edges)
    }
    fn source(&self, e: &Ed) -> Nd {
        e.0
    }
    fn target(&self, e: &Ed) -> Nd {
        e.1
    }
}
use crate::text_extractor::TextExtractor;
use bimap::BiMap;
use bit_vec::BitVec;
use parse_wiki_text::{DefinitionListItem, ListItem, Node, Output};
use regex::{Regex, RegexBuilder};
use serde::export::Formatter;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::Debug;

pub type Nd = usize;
pub type Ed = (Nd, Nd);

#[derive(Default, Debug)]
pub struct Graph {
    pub node_data: Vec<NodeData>,
    node_labels: BiMap<Nd, (String, bool)>,
    edge_labels: HashMap<Ed, String>,
}

#[derive(Default, Debug)]
pub struct NodeData {
    pub outgoing: Vec<usize>,
    pub incoming: Vec<usize>,
}

impl Graph {
    pub fn len(&self) -> usize {
        self.node_data.len()
    }

    pub fn add_vertex(&mut self, label: (String, bool)) -> Nd {
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

    pub fn add(&mut self, vtx1: (String, bool), edge: String, vtx2: (String, bool)) {
        let v1 = self.find_or_add_vertex(vtx1);
        let v2 = self.find_or_add_vertex(vtx2);
        self.add_edge((v1, v2), edge);
    }

    pub fn find_vertex(&self, label: &(String, bool)) -> Option<Nd> {
        self.node_labels.get_by_right(&label).map(|x| *x)
    }

    pub fn find_or_add_vertex(&mut self, label: (String,bool)) -> Nd {
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

    pub fn get_vertex_label(&self, id: Nd) -> &(String, bool) {
        self.node_labels.get_by_left(&id).unwrap()
    }

    pub fn get_edge_label(&self, e: &Ed) -> &str {
        self.edge_labels.get(e).unwrap()
    }

    /// Walk graph DFS post order.
    ///
    /// # Arguments
    /// * `start` - start node
    /// * `f` - visiting function
    ///
    /// # Return value
    /// Bit vector representing visited nodes.
    pub fn walk_dfs_post_order<F>(&self, start: Nd, mut f: F) -> Result<BitVec, Box<dyn Error>>
    where
        F: FnMut(Nd, &Vec<Nd>) -> Result<(), Box<dyn Error>>,
    {
        let mut visited = BitVec::from_elem(self.node_data.len(), false);
        let mut stack: Vec<(Nd, usize)> = Vec::new(); // (node, children_visited)
        let mut path: HashSet<usize> = HashSet::new();
        let mut edge_cuts: HashMap<usize, Vec<usize>> = HashMap::new();
        stack.push((start, 0));
        while !stack.is_empty() {
            let (node, children_visited) = stack.pop().unwrap();
            path.insert(node);
            // println!("pop {}", node);
            visited.set(node, true);
            if children_visited < self.node_data[node].outgoing.len() {
                stack.push((node, children_visited + 1));
                let next_child = self.node_data[node].outgoing[children_visited];
                if path.contains(&next_child) {
                    let node_label = self.get_vertex_label(node);
                    let child_label = self.get_vertex_label(next_child);
                    println!(
                        "Found loop between '{}' ({}) and '{}' ({})",
                        node_label.0, node, child_label.0, next_child
                    );
                    match edge_cuts.get_mut(&node) {
                        None => {
                            edge_cuts.insert(node, vec![next_child]);
                        }
                        Some(v) => {
                            v.push(next_child);
                        }
                    }
                }
                if !visited.get(next_child).unwrap() {
                    stack.push((next_child, 0));
                }
            } else {
                // all children are visited, so call function (post order)
                let empty: Vec<usize> = vec![];
                let forbidden = edge_cuts.get(&node).unwrap_or(&empty);
                f(node, forbidden)?;
                path.remove(&node);
            }
        }

        Ok(visited)
    }
}

#[derive(Default, Debug)]
pub struct CategoryExtractor {
    site: String,
    is_category: bool,
    pub graph: Graph,
    pub normalizer: Normalizer,
}

impl CategoryExtractor {
    pub fn set_site(&mut self, site: String) {
        self.site = site
    }

    pub fn set_is_category(&mut self, is_category: bool) {
        self.is_category = is_category;
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
                let target = self.normalizer.normalize_category_name(*target);
                if !target.1 {
                    panic!("Category target '{}' is not a category!", target.0);
                }
                // println!("TARGET: {}", target_name);
                let mut extr = TextExtractor::new();
                extr.extract_nodes_text(ordinal);
                let mut label = extr.result().trim().to_string();
                if label.is_empty()
                    || label.len() == 1 && !label.chars().next().unwrap().is_alphanumeric()
                {
                    label = self.site.clone();
                }
                self.graph.add(target, label, (self.site.clone(), self.is_category))
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
            kat_match: RegexBuilder::new(r"^(Kategoria|Category):")
                .case_insensitive(true)
                .build()
                .unwrap(),

            space_match: Regex::new(r"\s+").unwrap(),

            bad_chars: vec![left_to_right],
        }
    }
}

impl Normalizer {
    pub fn normalize_category_name(&self, s: &str) -> (String, bool) {
        let mut s = s;
        let is_category;
        if self.kat_match.is_match(s) {
            let i = s.find(':').unwrap()+1;
            s = &s[i..];
            is_category = true;
        } else {
            s = s;
            is_category = false;
        }
        s = s.trim();
        let s = self.space_match.replace_all(s, " ");
        let mut s = s.to_string();
        for ch in &self.bad_chars {
            s = s.replace(*ch, "");
        }
        (s,is_category)
    }
}

// Don't display it
impl Debug for Normalizer {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        Result::Ok(())
    }
}

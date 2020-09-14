use crate::text_extractor::TextExtractor;
use parse_wiki_text::{self, Node, Output};
use serde::Serialize;
use std::fmt;

#[derive(Serialize,Default)]
pub struct Cites {
    pub cites: Vec<Cite>,
}

#[derive(Serialize)]
pub struct Cite {
    pub text: String,
    pub sections: Vec<String>,
    pub meta: Vec<MetaData>,
}

impl Cite {
    pub fn new(text: String) -> Cite {
        Cite {
            text,
            sections: Vec::new(),
            meta: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MetaData {
    pub key: String,
    pub value: String,
    pub links: Vec<String>,
}

impl MetaData {
    pub fn new(key: String, value: String, links: Vec<String>) -> MetaData {
        MetaData { key, value, links }
    }
}

impl fmt::Display for Cite {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if !self.sections.is_empty() {
            writeln!(f, "[{}]", self.sections.join(" / "))?;
        }
        writeln!(f, "{}", self.text)?;
        for MetaData { key, value, .. } in &self.meta {
            writeln!(f, " * {}: {}", key, value)?;
        }
        fmt::Result::Ok(())
    }
}

impl Cites {
    pub fn extract_cites(&mut self, parsed: &Output, title: &str) {
        let mut breadcrumbs = Breadcrumbs::new(title);
        for node in &parsed.nodes {
            match node {
                Node::UnorderedList { items, .. } => {
                    for item in items {
                        let mut extr = TextExtractor::new();
                        extr.descend_lists = false;
                        extr.extract_item_text(&item);
                        let mut cite = Cite::new(extr.result());
                        cite.sections = breadcrumbs.stack.clone();

                        let mut meta_reader = MetaReader::default();
                        meta_reader.read(&item.nodes);
                        cite.meta = meta_reader.meta;

                        self.cites.push(cite);
                    }
                }

                Node::Heading { level, nodes, .. } => {
                    let mut extr = TextExtractor::new();
                    extr.extract_nodes_text(&nodes);
                    breadcrumbs.update(*level, extr.result())
                }

                _ => {}
            }
        }
    }
}

struct Breadcrumbs {
    stack: Vec<String>,
}

impl Breadcrumbs {
    pub fn new(title: &str) -> Breadcrumbs {
        Breadcrumbs {
            stack: vec![title.to_string()],
        }
    }

    pub fn update(&mut self, level: u8, text: String) {
        while self.stack.len() < level as usize {
            self.stack.push(String::new())
        }
        while self.stack.len() > level as usize {
            self.stack.pop();
        }
        let last = self.stack.len() - 1;
        if let Some(top) = self.stack.get_mut(last) {
            *top = text;
        }
    }
}

#[derive(Default)]
struct MetaReader {
    meta: Vec<MetaData>,
}

impl MetaReader {
    pub fn read(&mut self, items: &Vec<Node>) {
        for item in items {
            match item {
                Node::UnorderedList { items, .. } => {
                    for item in items {
                        let mut extr = TextExtractor::new();
                        extr.extract_item_text(item);
                        let text = extr.result();
                        let mut parts: Vec<&str> = text.splitn(2, ":").collect();
                        if parts.len() == 2 {
                            let second = parts.pop().unwrap().trim().to_string();
                            let first = parts.pop().unwrap().to_string();
                            self.meta.push(MetaData::new(first, second, vec![]));
                        }
                    }
                }

                _ => {}
            }
        }
    }
}

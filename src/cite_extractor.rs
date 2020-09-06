use crate::text_extractor::TextExtractor;
use parse_wiki_text::{self, Node, Output};
use std::fmt;

pub struct CiteExtractor {
    pub cites: Vec<Cite>,
}

pub struct Cite {
    pub text: String,
    pub sections: Vec<String>,
    pub meta: Vec<(String,String)>
}

impl Cite {
    pub fn new(text: String) -> Cite {
        Cite {
            text,
            sections: Vec::new(),
            meta: Vec::new()
        }
    }
}

impl fmt::Display for Cite {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if !self.sections.is_empty() {
            writeln!(f, "[{}]", self.sections.join(" / "))?;
        }
        write!(f, "{}", self.text)
    }
}

impl CiteExtractor {
    pub fn new() -> CiteExtractor {
        CiteExtractor { cites: Vec::new() }
    }

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
                        self.cites.push(cite);
                    }
                }

                Node::Heading {level, nodes, ..} => {
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
    stack: Vec<String>
}

impl Breadcrumbs {
    pub fn new(title: &str) -> Breadcrumbs {
        Breadcrumbs { stack: vec![title.to_string()] }
    }

    pub fn update(&mut self, level: u8, text: String) {
        while self.stack.len() < level as usize {
            self.stack.push(String::new())
        }
        while self.stack.len() > level as usize {
            self.stack.pop();
        }
        let last = self.stack.len()-1;
        if let Some(top) = self.stack.get_mut(last) {
            *top = text;
        }
    }
}
use super::text_extractor::TextExtractor;
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
        write!(f, "{}", self.text)
    }
}

impl CiteExtractor {
    pub fn new() -> CiteExtractor {
        CiteExtractor { cites: Vec::new() }
    }

    pub fn extract_cites(&mut self, parsed: Output) {
        for node in parsed.nodes {
            match node {
                Node::UnorderedList { items, .. } => {
                    for item in items {
                        let mut extr = TextExtractor::new();
                        extr.descend_lists = false;
                        extr.extract_item_text(item);
                        let cite = Cite::new(extr.result());
                        self.cites.push(cite);
                    }
                }

                Node::Heading {level, nodes, ..} => {
                    // TODO
                }

                _ => {}
            }
        }
    }
}

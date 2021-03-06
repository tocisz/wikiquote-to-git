use parse_wiki_text::{self, DefinitionListItem, ListItem, Node};

pub struct TextExtractor {
    pub text: Vec<String>,
    pub descend_lists: bool,
}

impl TextExtractor {
    pub fn new() -> TextExtractor {
        TextExtractor {
            text: Vec::new(),
            descend_lists: true,
        }
    }

    pub fn result(&self) -> String {
        self.text.join("")
    }

    /*    pub fn extract_text(&mut self, parsed: &Output) {
            for n in &parsed.nodes {
                self.extract_node_text(n)
            }
        }
    */
    pub fn extract_node_text(&mut self, node: &Node) {
        match node {
            Node::Heading { nodes, .. } => self.extract_nodes_text(nodes),

            Node::CharacterEntity { character, .. } => self.text.push(character.to_string()),

            Node::DefinitionList { items, .. } => {
                for n in items {
                    self.extract_dl_text(n)
                }
            }

            Node::Link { text, .. } => {
                // self.text.push("[".to_string());
                self.extract_nodes_text(text)
                // self.text.push("]".to_string());
            }

            Node::ExternalLink { nodes, .. } => {
                // self.text.push("[".to_string());
                self.extract_nodes_text(nodes)
                // self.text.push("]".to_string());
            }

            Node::Image { text, .. } => {
                self.text.push("[".to_string());
                self.extract_nodes_text(text);
                self.text.push("]".to_string());
            }

            Node::UnorderedList { items, .. } => {
                if self.descend_lists {
                    for n in items {
                        self.extract_item_text(n)
                    }
                }
            }

            Node::OrderedList { items, .. } => {
                if self.descend_lists {
                    for n in items {
                        self.extract_item_text(n)
                    }
                }
            }

            Node::Preformatted { nodes, .. } => self.extract_nodes_text(nodes),

            Node::Table { .. } => {
                // TODO?
            }

            Node::Tag { nodes, .. } => self.extract_nodes_text(nodes),

            Node::StartTag { name, .. } => {
                if name == "br" {
                    self.text.push("\n".to_string())
                }
            }

            Node::Text { value, .. } => self.text.push(value.to_string()),

            _ => {}
        }
    }

    pub fn extract_nodes_text(&mut self, nodes: &Vec<Node>) {
        for n in nodes {
            self.extract_node_text(n)
        }
    }

    pub fn extract_dl_text(&mut self, node: &DefinitionListItem) {
        for n in &node.nodes {
            self.extract_node_text(n)
        }
    }

    pub fn extract_item_text(&mut self, node: &ListItem) {
        for n in &node.nodes {
            self.extract_node_text(n)
        }
    }
}

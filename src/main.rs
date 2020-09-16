mod category_graph;
mod cite_extractor;
mod text_extractor;

use cite_extractor::Cites;
use parse_wiki_text::{self, Configuration, ConfigurationSource};
use radix_fmt::radix_36;
use structopt::StructOpt;

#[macro_use]
extern crate lazy_static;

// Configuration for pl.wikiquote.org
// Generated by https://github.com/portstrom/fetch_mediawiki_configuration
lazy_static! {
    static ref WIKICONF: Configuration = {
        Configuration::new(&ConfigurationSource {
            category_namespaces: &["category", "kategoria"],
            extension_tags: &[
                "categorytree",
                "ce",
                "charinsert",
                "chem",
                "dynamicpagelist",
                "gallery",
                "graph",
                "hiero",
                "imagemap",
                "indicator",
                "inputbox",
                "mapframe",
                "maplink",
                "math",
                "nowiki",
                "poem",
                "pre",
                "ref",
                "references",
                "score",
                "section",
                "source",
                "syntaxhighlight",
                "templatedata",
                "templatestyles",
                "timeline",
            ],
            file_namespaces: &["file", "grafika", "image", "plik"],
            link_trail: "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyzÓóĄąĆćĘęŁłŃńŚśŹźŻż",
            magic_words: &[
                "BEZEDYCJISEKCJI",
                "BEZGALERII",
                "BEZSPISU",
                "DISAMBIG",
                "EXPECTUNUSEDCATEGORY",
                "FORCETOC",
                "HIDDENCAT",
                "INDEKSUJ",
                "INDEX",
                "KATEGORIAUKRYTA",
                "LINKNOWEJSEKCJI",
                "NEWSECTIONLINK",
                "NIEINDEKSUJ",
                "NOCC",
                "NOCONTENTCONVERT",
                "NOEDITSECTION",
                "NOGALLERY",
                "NOGLOBAL",
                "NOINDEX",
                "NONEWSECTIONLINK",
                "NOTC",
                "NOTITLECONVERT",
                "NOTOC",
                "POZIOMZABEZPIECZEŃ",
                "SPIS",
                "STATICREDIRECT",
                "TOC",
                "WYMUŚSPIS",
                "ZESPISEM",
            ],
            protocols: &[
                "//",
                "bitcoin:",
                "ftp://",
                "ftps://",
                "geo:",
                "git://",
                "gopher://",
                "http://",
                "https://",
                "irc://",
                "ircs://",
                "magnet:",
                "mailto:",
                "mms://",
                "news:",
                "nntp://",
                "redis://",
                "sftp://",
                "sip:",
                "sips:",
                "sms:",
                "ssh://",
                "svn://",
                "tel:",
                "telnet://",
                "urn:",
                "worldwind://",
                "xmpp:",
            ],
            redirect_magic_words: &["PATRZ", "PRZEKIERUJ", "REDIRECT", "TAM"],
        })
    };
}

#[derive(Debug, PartialEq)]
enum Command {
    LIST,
    PARSE,
    JSON,
    DEBUG,
    CATS,
}

use crate::category_graph::{CategoryExtractor, Graph, Normalizer};
use bit_vec::BitVec;
use collecting_hashmap::CollectingHashMap;
use git2::{Oid, Repository, Signature};
use parse_mediawiki_dump::Page;
use serde::export::Formatter;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Display;
use std::str::FromStr;
use std::string::ParseError;

impl FromStr for Command {
    type Err = ParseError;
    fn from_str(day: &str) -> Result<Self, Self::Err> {
        match day {
            "list" => Ok(Command::LIST),
            "parse" => Ok(Command::PARSE),
            "json" => Ok(Command::JSON),
            "debug" => Ok(Command::DEBUG),
            "cats" => Ok(Command::CATS),
            _ => Ok(Command::LIST),
        }
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "wikiquote", about = "Parse wikiquotes.")]
struct Opt {
    #[structopt(short = "c", default_value = "parse")]
    command: Command,

    #[structopt(short = "d")]
    datafile: String,

    #[structopt(short = "o")]
    output: String,

    #[structopt(default_value)]
    search: String,
}

fn main() {
    let args: Opt = Opt::from_args();

    match do_main(args) {
        Ok(()) => {}
        Err(e) => eprintln!("ERROR: {}", e),
    }
}

struct CategoryData(Graph, category_graph::Nd, BitVec);

fn do_main(args: Opt) -> Result<(), Box<dyn Error>> {
    if args.command == Command::CATS {
        let repo = Repository::init(&args.output)?;
        let cat_data = process_categories(&args, get_reader(&args)?)?;
        let cite_hashes = add_articles_to_git(&cat_data, get_reader(&args)?, &repo)?;
        store_categories_in_git(&cat_data, cite_hashes, repo)?;
    } else {
        add_articles(&args, get_reader(&args)?)?;
    }
    Ok(())
}

fn get_reader(cfg: &Opt) -> Result<Box<dyn std::io::BufRead>, Box<dyn Error>> {
    let file = std::io::BufReader::new(std::fs::File::open(&cfg.datafile)?);

    let reader: Box<dyn std::io::BufRead> = if cfg.datafile.ends_with(".bz2") {
        Box::new(std::io::BufReader::new(bzip2::bufread::BzDecoder::new(
            file,
        )))
    } else {
        Box::new(file)
    };

    Result::Ok(reader)
}

#[derive(Debug, Default)]
struct NoRootCategoryError;

impl Display for NoRootCategoryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "No root category found!")
    }
}

impl Error for NoRootCategoryError {}

#[derive(Debug)]
struct MediawikiParseError(parse_mediawiki_dump::Error);

impl Display for MediawikiParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Mediawiki parse error: ")?;
        self.0.fmt(f)
    }
}

impl Error for MediawikiParseError {}

fn process_categories(
    args: &Opt,
    source: impl std::io::BufRead,
) -> Result<CategoryData, Box<dyn Error>> {
    let mut category_extractor = CategoryExtractor::default();
    for result in parse_mediawiki_dump::parse(source) {
        match result {
            Err(error) => return Err(Box::new(MediawikiParseError(error))),
            Ok(page) => {
                let (site_name, is_category) = category_extractor
                    .normalizer
                    .normalize_category_name(&page.title);
                let parsed = WIKICONF.parse(&page.text);
                category_extractor.set_site(site_name);
                category_extractor.set_is_category(is_category);
                category_extractor.extract(&parsed);
            }
        }
    }

    let found_root = if !args.search.is_empty() {
        let search = (args.search.clone(), true);
        match category_extractor.graph.find_vertex(&search) {
            None => {
                let roots = category_extractor.graph.roots();
                roots.get(0).map(|x| *x)
            }
            some => some,
        }
    } else {
        let roots = category_extractor.graph.roots();
        roots.get(0).map(|x| *x)
    };

    if let Some(root) = found_root {
        let visited = category_extractor
            .graph
            .walk_dfs_post_order(root, |_, _| Result::Ok(()))?;

        println!(
            "Visited {} out of {} nodes.",
            count_ones(&visited),
            category_extractor.graph.len()
        );

        Result::Ok(CategoryData(category_extractor.graph, root, visited))
    } else {
        Result::Err(Box::new(NoRootCategoryError::default()))
    }
}

fn add_articles(args: &Opt, source: impl std::io::BufRead) -> Result<(), Box<dyn Error>> {
    for result in parse_mediawiki_dump::parse(source) {
        match result {
            Err(error) => {
                eprintln!("Error: {}", error);
                std::process::exit(1);
            }
            Ok(page) => match args.command {
                Command::LIST => {
                    println!(
                        "{} {} {:?} {:?}",
                        page.namespace, page.title, page.format, page.model
                    );
                }

                Command::PARSE | Command::JSON => {
                    if page.title == args.search {
                        println!(
                            "{} {} {:?} {:?}",
                            page.namespace, page.title, page.format, page.model
                        );
                        let parsed = WIKICONF.parse(&page.text);
                        let mut extr = Cites::default();
                        extr.extract_cites(&parsed, &page.title);
                        if args.command == Command::PARSE {
                            for cite in extr.cites {
                                println!("{}", cite);
                            }
                        } else {
                            let ser = serde_json::to_string_pretty(&extr).unwrap();
                            println!("{}", ser);
                        }
                    }
                }

                Command::DEBUG => {
                    if page.title == args.search {
                        println!(
                            "{} {} {:?} {:?}",
                            page.namespace, page.title, page.format, page.model
                        );
                        let parsed = WIKICONF.parse(&page.text);
                        println!("{:?}\n", parsed);
                    }
                }

                _ => {}
            },
        }
    }

    Result::Ok(())
}

type CiteHashes = CollectingHashMap<category_graph::Nd, Oid>;

fn add_articles_to_git(
    cat_data: &CategoryData,
    source: impl std::io::BufRead,
    repo: &Repository,
) -> Result<CiteHashes, Box<dyn Error>> {
    let mut result: CiteHashes = CollectingHashMap::new();
    let CategoryData(graph, _root, _visited) = cat_data;
    let normalizer = Normalizer::default();
    for parsed in parse_mediawiki_dump::parse(source) {
        match parsed {
            Err(error) => {
                eprintln!("Error: {}", error);
                std::process::exit(1);
            }
            Ok(Page {
                   format: p_format,
                   model: p_model,
                   namespace: p_ns,
                   text: p_text,
                   title: p_title,
               }) => {
                if p_ns == 0 && p_format.is_some() && p_model.is_some() {
                    let p_format = p_format.unwrap();
                    let p_model = p_model.unwrap();
                    if p_format == "text/x-wiki" && p_model == "wikitext" {
                        let cat = normalizer.normalize_category_name(&p_title);
                        if !cat.1 {
                            if let Some(v) = graph.find_vertex(&cat) {
                                println!("{}", p_title);
                                let parsed = WIKICONF.parse(&p_text);
                                let mut extr = Cites::default();
                                extr.extract_cites(&parsed, &p_title);
                                for cite in extr.cites {
                                    let out = format!("{}", cite);
                                    let id = repo.blob(out.as_bytes())?;
                                    result.insert(v, id);
                                }
                            }
                        }
                    } else {
                        println!(
                            "Skip {} {} {:?} {:?}",
                            p_ns, p_title, p_format, p_model
                        );
                    }
                } else {
                    println!(
                        "Skip {} {} {:?} {:?}",
                        p_ns, p_title, p_format, p_model
                    );
                }
            }
        }
    }
    Result::Ok(result)
}

fn store_categories_in_git(
    cat_data: &CategoryData,
    cite_hashes: CiteHashes,
    repo: Repository,
) -> Result<(), Box<dyn Error>> {
    let CategoryData(graph, root, _visited) = cat_data;

    let mut hashes: HashMap<category_graph::Nd, Oid> = HashMap::new();

    let _visited = graph.walk_dfs_post_order(*root, |n, forbidden| {
        let v_label = graph.get_vertex_label(n);
        let name_blob = repo.blob(v_label.0.as_bytes())?;
        let mut builder = repo.treebuilder(None)?;
        let blob_name = if v_label.1 { "cat.txt" } else { "art.txt" };
        builder.insert(blob_name, name_blob, 0o100644)?;
        let data = &graph.node_data[n];
        for out in &data.outgoing {
            if !forbidden.contains(out) {
                let name = get_git_file_name(&graph, n, *out);
                let h = hashes.get(out).expect("Children should be already added");
                builder.insert(name, *h, 0o040000)?;
            }
        }
        if let Some(cites) = cite_hashes.get_all(&n) {
            let mut i = 0u32;
            for c in cites {
                i += 1;
                let cname = format!("{}.txt", radix_36(i));
                builder.insert(cname, *c, 0o100644)?;
            }
        }
        let tree = builder.write()?;
        hashes.insert(n, tree);
        Ok(())
    })?;

    let root_h = hashes.get(&root).unwrap();
    let root_t = repo.find_tree(*root_h)?;
    let signature = Signature::now("WikiQuotes", "anonymous@pl.wikiquote.org")?;
    let commit = repo.commit(None, &signature, &signature, "init repo", &root_t, &[])?;
    println!("commit is {}", commit.to_string());

    let c = repo.find_commit(commit)?;
    repo.branch("master", &c, false)?;

    Ok(())
}

fn get_git_file_name(graph: &Graph, from: category_graph::Nd, to: category_graph::Nd) -> String {
    let el = graph.get_edge_label(&(from, to));
    let name = if !el.is_empty() {
        el
    } else {
        graph.get_vertex_label(to).0.as_ref()
    };
    name.replace("/", "-")
}

fn count_ones(visited: &BitVec) -> usize {
    let mut cnt: usize = 0;
    for b in visited.blocks() {
        cnt += b.count_ones() as usize;
    }
    cnt
}

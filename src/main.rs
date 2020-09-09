mod category_graph;
mod cite_extractor;
mod text_extractor;

use cite_extractor::Cites;
use parse_wiki_text::{self, Configuration, ConfigurationSource};
use structopt::StructOpt;

// Configuration for pl.wikiquote.org
pub fn create_configuration() -> Configuration {
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
}

#[derive(Debug, PartialEq)]
enum Command {
    LIST,
    PARSE,
    JSON,
    DEBUG,
    CATS
}

use std::str::FromStr;
use std::string::ParseError;
use crate::category_graph::CategoryExtractor;
use std::error::Error;

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
    #[structopt(short = "c", long, default_value = "parse")]
    command: Command,

    #[structopt(default_value)]
    search: String,
}

#[derive(Debug)]
struct Config {
    datafile: String,
}

impl ::std::default::Default for Config {
    fn default() -> Self {
        Self {
            datafile: "plwikiquote-20200820-pages-articles.xml.bz2".to_string(),
        }
    }
}

fn main() {
    let cfg = Config::default();
    let args: Opt = Opt::from_args();

    let file = match std::fs::File::open(&cfg.datafile) {
        Err(error) => {
            eprintln!("Failed to open input file: {}", error);
            std::process::exit(1);
        }
        Ok(file) => std::io::BufReader::new(file),
    };
    let result = if cfg.datafile.ends_with(".bz2") {
        parse(
            args,
            std::io::BufReader::new(bzip2::bufread::BzDecoder::new(file)),
        )
    } else {
        parse(args, file)
    };

    match result {
        Ok(()) => {}
        Err(e) => eprintln!("ERROR: {}", e)
    }
}

fn parse(args: Opt, source: impl std::io::BufRead) -> Result<(),Box<dyn Error>> {
    let mut category_extractor = CategoryExtractor::default();

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
                        let parsed = create_configuration().parse(&page.text);
                        let mut extr = Cites::new();
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

                Command::CATS => {
                    if page.title.starts_with("Kategoria:") {
                        let site_name = category_graph::after_colon(&page.title);
                        // println!("SITE: {}", site_name);
                        let parsed = create_configuration().parse(&page.text);
                        category_extractor.set_site(site_name);
                        category_extractor.extract(&parsed);
                    }
                }

                Command::DEBUG => {
                    if page.title == args.search {
                        println!(
                            "{} {} {:?} {:?}",
                            page.namespace, page.title, page.format, page.model
                        );
                        let parsed = create_configuration().parse(&page.text);
                        println!("{:?}\n", parsed);
                    }
                }
            },
        }
    }

    if args.command == Command::CATS {
        //println!("{:?}", category_extractor);
        use std::fs::File;
        let mut f = File::create("cats.dot").unwrap();
        dot::render(&category_extractor.graph, &mut f)?;
    }

    Result::Ok(())
}

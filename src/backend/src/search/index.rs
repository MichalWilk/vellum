use std::path::Path;

use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::snippet::SnippetGenerator;
use tantivy::{doc, Index, IndexReader, ReloadPolicy, TantivyDocument};

pub struct SearchIndex {
    index: Index,
    reader: IndexReader,
    path_field: Field,
    title_field: Field,
    body_field: Field,
}

pub struct SearchResult {
    pub path: String,
    pub title: String,
    pub snippet: String,
    pub score: f32,
}

impl SearchIndex {
    pub fn build(vault_root: &Path) -> anyhow::Result<Self> {
        let mut schema_builder = Schema::builder();

        let path_field = schema_builder.add_text_field("path", STRING | STORED);
        let title_field = schema_builder.add_text_field("title", TEXT | STORED);
        let body_field = schema_builder.add_text_field("body", TEXT | STORED);

        let schema = schema_builder.build();
        let index = Index::create_in_ram(schema);

        let mut writer = index.writer(50_000_000)?;

        crate::docs::walk::walk_vault_files(vault_root, |rel_path, full_path| {
            let content = std::fs::read_to_string(full_path).unwrap_or_default();
            let (title, body) = extract_title_and_body(&content);

            let _ = writer.add_document(doc!(
                path_field => rel_path.to_string(),
                title_field => title,
                body_field => body,
            ));
        });

        writer.commit()?;

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;

        Ok(Self {
            index,
            reader,
            path_field,
            title_field,
            body_field,
        })
    }

    pub fn search(&self, query_str: &str, limit: usize) -> Vec<SearchResult> {
        let searcher = self.reader.searcher();
        let mut query_parser = QueryParser::for_index(
            &self.index,
            vec![self.title_field, self.body_field],
        );
        query_parser.set_field_boost(self.title_field, 2.0);

        let query = match query_parser.parse_query(query_str) {
            Ok(q) => q,
            Err(_) => return Vec::new(),
        };

        let top_docs = match searcher.search(&query, &TopDocs::with_limit(limit)) {
            Ok(docs) => docs,
            Err(_) => return Vec::new(),
        };

        let snippet_generator = SnippetGenerator::create(&searcher, &*query, self.body_field);

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let retrieved: TantivyDocument = match searcher.doc(doc_address) {
                Ok(d) => d,
                Err(_) => continue,
            };

            let path = doc_field_text(&retrieved, self.path_field);
            let title = doc_field_text(&retrieved, self.title_field);

            let snippet = match &snippet_generator {
                Ok(gen) => {
                    let snip = gen.snippet_from_doc(&retrieved);
                    snip.to_html()
                }
                Err(_) => String::new(),
            };

            results.push(SearchResult {
                path,
                title,
                snippet,
                score,
            });
        }

        results
    }
}

fn doc_field_text(doc: &TantivyDocument, field: Field) -> String {
    doc.get_first(field)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn extract_title_and_body(content: &str) -> (String, String) {
    let (title, body_start) = if content.starts_with("---") {
        if let Some(end) = content[3..].find("\n---") {
            let yaml = &content[3..3 + end];
            let title = yaml
                .lines()
                .find(|l| l.trim_start().starts_with("title:"))
                .map(|l| {
                    l.trim_start()
                        .strip_prefix("title:")
                        .unwrap_or("")
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'')
                        .to_string()
                })
                .unwrap_or_default();
            let body = &content[3 + end + 4..];
            (title, body)
        } else {
            (String::new(), content)
        }
    } else {
        (String::new(), content)
    };

    let plain = strip_markdown(body_start);

    let title = if title.is_empty() {
        body_start
            .lines()
            .find(|l| l.starts_with("# "))
            .map(|l| l.trim_start_matches('#').trim().to_string())
            .unwrap_or_default()
    } else {
        title
    };

    (title, plain)
}

fn strip_markdown(md: &str) -> String {
    use pulldown_cmark::{Event, Parser};

    let parser = Parser::new(md);
    let mut text = String::new();

    for event in parser {
        match event {
            Event::Text(t) | Event::Code(t) => {
                text.push_str(&t);
                text.push(' ');
            }
            Event::SoftBreak | Event::HardBreak => {
                text.push(' ');
            }
            _ => {}
        }
    }

    text
}

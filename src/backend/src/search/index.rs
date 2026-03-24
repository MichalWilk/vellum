use std::path::Path;

use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::snippet::SnippetGenerator;
use tantivy::tokenizer::NgramTokenizer;
use tantivy::{doc, Index, IndexReader, ReloadPolicy, TantivyDocument};

pub struct SearchIndex {
    index: Index,
    reader: IndexReader,
    path_field: Field,
    title_field: Field,
    body_field: Field,
    tags_field: Field,
    headings_text_field: Field,
    headings_data_field: Field,
}

pub struct SearchResult {
    pub path: String,
    pub title: String,
    pub snippet: String,
    pub score: f32,
}

pub struct HeadingSearchResult {
    pub path: String,
    pub title: String,
    pub headings_data: String,
    pub score: f32,
}

pub struct TagSearchResult {
    pub path: String,
    pub tags: String,
    pub score: f32,
}

impl SearchIndex {
    pub fn build(vault_root: &Path) -> anyhow::Result<Self> {
        let mut schema_builder = Schema::builder();

        let ngram_options = TextOptions::default()
            .set_indexing_options(
                TextFieldIndexing::default()
                    .set_tokenizer("ngram")
                    .set_index_option(IndexRecordOption::WithFreqsAndPositions),
            )
            .set_stored();

        let path_field = schema_builder.add_text_field("path", ngram_options);
        let title_field = schema_builder.add_text_field("title", TEXT | STORED);
        let body_field = schema_builder.add_text_field("body", TEXT | STORED);
        let tags_field = schema_builder.add_text_field("tags", TEXT | STORED);
        let headings_text_field = schema_builder.add_text_field("headings_text", TEXT);
        let headings_data_field = schema_builder.add_text_field("headings_data", STORED);

        let schema = schema_builder.build();
        let index = Index::create_in_ram(schema);

        index
            .tokenizers()
            .register("ngram", NgramTokenizer::new(2, 20, false)?);

        let mut writer = index.writer(50_000_000)?;

        crate::docs::walk::walk_vault_files(vault_root, |rel_path, full_path| {
            let content = std::fs::read_to_string(full_path).unwrap_or_default();
            let (title, body) = extract_title_and_body(&content);

            let tags_list = crate::docs::markdown::extract_tags(&content);
            let tags_text = tags_list
                .iter()
                .map(|t| t.to_lowercase())
                .collect::<Vec<_>>()
                .join(" ");

            let headings = crate::docs::markdown::extract_headings(&content);
            let headings_text = headings
                .iter()
                .map(|h| h.text.clone())
                .collect::<Vec<_>>()
                .join("\n");
            let headings_json = serde_json::to_string(&headings).unwrap_or_default();

            let _ = writer.add_document(doc!(
                path_field => rel_path.to_string(),
                title_field => title,
                body_field => body,
                tags_field => tags_text,
                headings_text_field => headings_text,
                headings_data_field => headings_json,
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
            tags_field,
            headings_text_field,
            headings_data_field,
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

    pub fn search_files(&self, query_str: &str, limit: usize) -> Vec<SearchResult> {
        let searcher = self.reader.searcher();
        let query_parser = QueryParser::for_index(&self.index, vec![self.path_field]);

        let query = match query_parser.parse_query(query_str) {
            Ok(q) => q,
            Err(_) => return Vec::new(),
        };

        let top_docs = match searcher.search(&query, &TopDocs::with_limit(limit)) {
            Ok(docs) => docs,
            Err(_) => return Vec::new(),
        };

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let retrieved: TantivyDocument = match searcher.doc(doc_address) {
                Ok(d) => d,
                Err(_) => continue,
            };

            let path = doc_field_text(&retrieved, self.path_field);
            let title = doc_field_text(&retrieved, self.title_field);

            results.push(SearchResult {
                path,
                title,
                snippet: String::new(),
                score,
            });
        }

        results
    }

    pub fn search_tags(&self, query_str: &str, limit: usize) -> Vec<TagSearchResult> {
        let searcher = self.reader.searcher();
        let query_parser = QueryParser::for_index(&self.index, vec![self.tags_field]);

        let query = match query_parser.parse_query(query_str) {
            Ok(q) => q,
            Err(_) => return Vec::new(),
        };

        let top_docs = match searcher.search(&query, &TopDocs::with_limit(limit)) {
            Ok(docs) => docs,
            Err(_) => return Vec::new(),
        };

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let retrieved: TantivyDocument = match searcher.doc(doc_address) {
                Ok(d) => d,
                Err(_) => continue,
            };

            let path = doc_field_text(&retrieved, self.path_field);
            let tags = doc_field_text(&retrieved, self.tags_field);

            results.push(TagSearchResult { path, tags, score });
        }

        results
    }

    pub fn search_headings(&self, query_str: &str, limit: usize) -> Vec<HeadingSearchResult> {
        let searcher = self.reader.searcher();
        let query_parser = QueryParser::for_index(&self.index, vec![self.headings_text_field]);

        let query = match query_parser.parse_query(query_str) {
            Ok(q) => q,
            Err(_) => return Vec::new(),
        };

        let top_docs = match searcher.search(&query, &TopDocs::with_limit(limit)) {
            Ok(docs) => docs,
            Err(_) => return Vec::new(),
        };

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let retrieved: TantivyDocument = match searcher.doc(doc_address) {
                Ok(d) => d,
                Err(_) => continue,
            };

            let path = doc_field_text(&retrieved, self.path_field);
            let title = doc_field_text(&retrieved, self.title_field);
            let headings_data = doc_field_text(&retrieved, self.headings_data_field);

            results.push(HeadingSearchResult {
                path,
                title,
                headings_data,
                score,
            });
        }

        results
    }

    pub fn search_by_tag(&self, tag: &str, limit: usize) -> Vec<TagSearchResult> {
        use tantivy::query::TermQuery;

        let searcher = self.reader.searcher();
        let term = tantivy::Term::from_field_text(self.tags_field, tag);
        let query = TermQuery::new(term, IndexRecordOption::Basic);

        let top_docs = match searcher.search(&query, &TopDocs::with_limit(limit)) {
            Ok(docs) => docs,
            Err(_) => return Vec::new(),
        };

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let retrieved: TantivyDocument = match searcher.doc(doc_address) {
                Ok(d) => d,
                Err(_) => continue,
            };

            let path = doc_field_text(&retrieved, self.path_field);
            let tags = doc_field_text(&retrieved, self.tags_field);

            results.push(TagSearchResult { path, tags, score });
        }

        results
    }

    pub fn all_tags(&self) -> Vec<TagSearchResult> {
        use tantivy::query::AllQuery;

        let searcher = self.reader.searcher();
        let limit = searcher.num_docs() as usize;
        if limit == 0 {
            return Vec::new();
        }

        let top_docs = match searcher.search(&AllQuery, &TopDocs::with_limit(limit)) {
            Ok(docs) => docs,
            Err(_) => return Vec::new(),
        };

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let retrieved: TantivyDocument = match searcher.doc(doc_address) {
                Ok(d) => d,
                Err(_) => continue,
            };

            let path = doc_field_text(&retrieved, self.path_field);
            let tags = doc_field_text(&retrieved, self.tags_field);

            if !tags.is_empty() {
                results.push(TagSearchResult { path, tags, score });
            }
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

use crate::Parser;
use anyhow::{Context, Result};
use qlty_types::tests::v1::FileCoverage;
use serde::Deserialize;
use serde_xml_rs;

#[derive(Debug, Deserialize)]
#[serde(rename = "report")]
struct JacocoSource {
    package: Vec<Package>,
}

#[derive(Debug, Deserialize)]
struct Package {
    name: String,
    sourcefile: Vec<Sourcefile>,
}

#[derive(Debug, Deserialize)]
struct Sourcefile {
    name: String,
    line: Vec<Line>,
}

#[derive(Debug, Deserialize)]
struct Line {
    nr: i64,
    ci: i64,
}

pub struct Jacoco {}

impl Jacoco {
    pub fn new() -> Self {
        Self {}
    }
}

impl Parser for Jacoco {
    fn parse_text(&self, text: &str) -> Result<Vec<FileCoverage>> {
        let source: JacocoSource =
            serde_xml_rs::from_str(text).with_context(|| "Failed to parse XML text")?;
        let mut file_coverages: Vec<FileCoverage> = vec![];

        for package in source.package.iter() {
            for sourcefile in package.sourcefile.iter() {
                let mut line_hits = Vec::new();
                for line in sourcefile.line.iter() {
                    // Fill in any missing lines with -1 to indicate that are omitted
                    for _x in (line_hits.len() as i64)..(line.nr - 1) {
                        line_hits.push(-1)
                    }

                    line_hits.push(line.ci);
                }

                let path = format!("{}/{}", package.name, sourcefile.name);

                let file_coverage = FileCoverage {
                    path,
                    hits: line_hits,
                    ..Default::default()
                };

                file_coverages.push(file_coverage);
            }
        }

        Ok(file_coverages)
    }
}
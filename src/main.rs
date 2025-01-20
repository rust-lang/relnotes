use std::collections::BTreeMap;
use std::collections::HashMap;
use std::env;

use askama::Template;
use chrono::prelude::*;
use chrono::Duration;

use reqwest::header::HeaderMap;
use serde_json as json;

const SKIP_LABELS: &[&str] = &["rollup"];

const RELNOTES_LABELS: &[&str] = &[
    "relnotes",
    "relnotes-perf",
    "finished-final-comment-period",
    "needs-fcp",
];

#[derive(Clone, Template)]
#[template(path = "relnotes.md", escape = "none")]
struct ReleaseNotes {
    version: String,
    date: NaiveDate,

    language_relnotes: String,
    compiler_relnotes: String,
    libraries_relnotes: String,
    stabilized_apis_relnotes: String,
    const_stabilized_apis_relnotes: String,
    cargo_relnotes: String,
    rustdoc_relnotes: String,
    compat_relnotes: String,
    internal_changes_relnotes: String,
    other_relnotes: String,
}

fn main() {
    let mut args = env::args();
    let _ = args.next();
    let version = args
        .next()
        .expect("A version number (xx.yy.zz) for the Rust release is required.");
    let today = Utc::now().date_naive();

    // A known rust release date. (1.42.0)
    let mut end = Utc
        .with_ymd_and_hms(2020, 3, 12, 0, 0, 0)
        .single()
        .unwrap()
        .date_naive();
    let six_weeks = Duration::weeks(6);

    // Get the most recent rust release date.
    while today - end > six_weeks {
        end = end + six_weeks;
    }

    let issues = get_issues_by_milestone(&version, "rust");
    let mut tracking_rust = TrackingIssues::collect(&issues);

    // Skips some PRs that aren't themselves interesting.
    let in_release = issues.iter().filter(|v| {
        !v["labels"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| SKIP_LABELS.contains(&o["name"].as_str().unwrap()))
    });

    let relnotes = in_release
        .into_iter()
        .filter(|o| has_tags(o, RELNOTES_LABELS))
        .collect::<Vec<_>>();

    let Sections {
        language_relnotes,
        compiler_relnotes,
        libraries_relnotes,
        stabilized_apis_relnotes,
        const_stabilized_apis_relnotes,
        rustdoc_relnotes,
        compat_relnotes,
        internal_changes_relnotes,
        other_relnotes,
    } = to_sections(relnotes, &mut tracking_rust);

    let cargo_issues = get_issues_by_milestone(&version, "cargo");

    let cargo_relnotes = {
        let relnotes = cargo_issues
            .iter()
            .filter(|o| has_tags(o, RELNOTES_LABELS))
            .collect::<Vec<_>>();

        relnotes
            .iter()
            .map(|o| {
                format!(
                    "- [{title}]({url}/)",
                    title = o["title"].as_str().unwrap(),
                    url = o["url"].as_str().unwrap(),
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    for issue in tracking_rust.issues.values() {
        for (section, (used, _)) in issue.sections.iter() {
            if issue.raw["labels"]["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|l| l["name"] == "relnotes-tracking-issue")
                && issue.raw["state"] == "CLOSED"
            {
                assert!(
                    !*used,
                    "{} <{}> used despite tracking issue being closed",
                    issue.raw["title"].as_str().unwrap(),
                    issue.raw["url"].as_str().unwrap()
                );
                continue;
            }

            if *used {
                continue;
            }

            eprintln!(
                "Did not use {:?} from {} <{}> (intended to provide relnotes for: {:?})",
                section,
                issue.raw["title"].as_str().unwrap(),
                issue.raw["url"].as_str().unwrap(),
                issues
                    .iter()
                    .find(|i| i["number"].as_u64().unwrap() == issue.for_number)
            );
        }
    }

    let relnotes = ReleaseNotes {
        version,
        date: end + six_weeks,

        language_relnotes,
        compiler_relnotes,
        libraries_relnotes,
        rustdoc_relnotes,
        stabilized_apis_relnotes,
        const_stabilized_apis_relnotes,
        cargo_relnotes,
        internal_changes_relnotes,
        compat_relnotes,
        other_relnotes,
    };

    println!("{}", relnotes.render().unwrap());
}

fn get_issues_by_milestone(version: &str, repo_name: &'static str) -> Vec<json::Value> {
    let mut out = get_issues_by_milestone_inner(version, repo_name, "issues");
    out.extend(get_issues_by_milestone_inner(
        version,
        repo_name,
        "pullRequests",
    ));
    out.sort_unstable_by_key(|v| v["number"].as_u64().unwrap());
    out.dedup_by_key(|v| v["number"].as_u64().unwrap());
    out
}

fn get_issues_by_milestone_inner(
    version: &str,
    repo_name: &'static str,
    ty: &str,
) -> Vec<json::Value> {
    use reqwest::blocking::Client;

    let headers = request_header();
    let mut args = BTreeMap::new();
    if ty == "pullRequests" {
        args.insert("states", String::from("[MERGED]"));
    }
    args.insert("last", String::from("100"));
    let mut issues = Vec::new();

    loop {
        let query = format!(
            r#"
            query {{
                repository(owner: "rust-lang", name: "{repo_name}") {{
                    milestones(query: "{version}", first: 1) {{
                        totalCount
                        nodes {{
                            {ty}({args}) {{
                                nodes {{
                                    number
                                    title
                                    url
                                    body
                                    state
                                    labels(last: 100) {{
                                        nodes {{
                                            name
                                        }}
                                    }}
                                }}
                                pageInfo {{
                                    startCursor
                                }}
                            }}
                        }}
                    }}
                }}
            }}"#,
            repo_name = repo_name,
            version = version,
            ty = ty,
            args = args
                .iter()
                .map(|(k, v)| format!("{}: {}", k, v))
                .collect::<Vec<_>>()
                .join(",")
        )
        .replace(" ", "")
        .replace("\n", " ")
        .replace('"', "\\\"");

        let json_query = format!("{{\"query\": \"{}\"}}", query);

        let client = Client::new();

        let response = client
            .post("https://api.github.com/graphql")
            .headers(headers.clone())
            .body(json_query)
            .send()
            .unwrap();
        let status = response.status();
        let json = response.json::<json::Value>().unwrap();
        if !status.is_success() {
            panic!("API Error {}: {}", status, json);
        }

        let milestones_data = json["data"]["repository"]["milestones"].clone();
        assert_eq!(
            milestones_data["totalCount"].as_u64().unwrap(),
            1,
            "More than one milestone matched the query \"{version}\". Please be more specific.",
            version = version
        );
        let pull_requests_data = milestones_data["nodes"][0][ty].clone();

        let mut pull_requests = pull_requests_data["nodes"].as_array().unwrap().clone();
        issues.append(&mut pull_requests);

        match &pull_requests_data["pageInfo"]["startCursor"] {
            json::Value::String(cursor) => {
                args.insert("before", format!("\"{}\"", cursor));
            }
            json::Value::Null => {
                break issues;
            }
            _ => unreachable!(),
        }
    }
}

fn request_header() -> HeaderMap {
    use reqwest::header::*;
    let token = env::var("GITHUB_TOKEN").expect("Set GITHUB_TOKEN to a valid token");
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert(ACCEPT, "application/json".parse().unwrap());
    headers.insert(AUTHORIZATION, format!("Bearer {}", token).parse().unwrap());
    headers.insert(USER_AGENT, "Rust-relnotes/0.1.0".parse().unwrap());
    headers
}

struct TrackingIssues {
    // Maps the issue/PR number *tracked* by the issue in `json::Value`.
    //
    // bool is tracking whether we've used that issue already.
    issues: HashMap<u64, TrackingIssue>,
}

#[derive(Debug)]
struct TrackingIssue {
    for_number: u64,
    raw: json::Value,
    // Section name -> (used, lines)
    sections: HashMap<String, (bool, Vec<String>)>,
}

impl TrackingIssues {
    fn collect(all: &[json::Value]) -> Self {
        const PREFIX: &str = "Tracking issue for release notes of #";
        const MARKDOWN: &str = r"(?ms)^(`{3,})markdown\r?\n(.*?)^\1\r?\n";

        let markdown_re = fancy_regex::Regex::new(MARKDOWN).unwrap();
        let mut tracking_issues = HashMap::new();
        for o in all.iter() {
            let title = o["title"].as_str().unwrap();
            if let Some(tail) = title.strip_prefix(PREFIX) {
                let for_number = tail[..tail.find(':').unwrap()].parse::<u64>().unwrap();
                let mut sections = HashMap::new();
                let body = o["body"].as_str().unwrap();

                let Ok(Some(captures)) = markdown_re.captures(body) else {
                    let issue = &o["number"];
                    eprintln!("WARNING: skipping {issue}, markdown not found:");
                    eprintln!("  {body:?}");
                    continue;
                };
                let relnotes = &captures[2];

                let mut in_section = None;
                for line in relnotes.lines() {
                    if line.trim().is_empty() {
                        continue;
                    }

                    if let Some(header) = line.strip_prefix("# ") {
                        in_section = Some(header);
                        continue;
                    }

                    if let Some(section) = in_section {
                        sections
                            .entry(section.to_owned())
                            .or_insert_with(|| (false, vec![]))
                            .1
                            .push(line.to_owned());
                    }
                }
                tracking_issues.insert(
                    for_number,
                    TrackingIssue {
                        for_number,
                        raw: o.clone(),
                        sections,
                    },
                );
            }
        }
        Self {
            issues: tracking_issues,
        }
    }
}

fn map_to_line_items<'a>(
    iter: impl IntoIterator<Item = &'a json::Value>,
    tracking_issues: &mut TrackingIssues,
    by_section: &mut HashMap<&'static str, String>,
) {
    for o in iter {
        let title = o["title"].as_str().unwrap();
        if title.starts_with("Tracking issue for release notes of #") {
            continue;
        }
        let number = o["number"].as_u64().unwrap();

        if let Some(issue) = tracking_issues.issues.get_mut(&number) {
            // If the tracking issue is closed, skip.
            if issue.raw["state"] == "CLOSED" {
                continue;
            }

            for (section, (used, lines)) in issue.sections.iter_mut() {
                if let Some((_, contents)) = by_section
                    .iter_mut()
                    .find(|(s, _)| s.eq_ignore_ascii_case(section))
                {
                    *used = true;
                    for line in lines.iter() {
                        contents.push_str(line);
                        contents.push('\n');
                    }
                }
            }

            // If we have a dedicated tracking issue, don't use our default rules.
            continue;
        }

        // In the future we expect to have increasingly few things fall into this category, as
        // things are added to the dedicated tracking issue category in triagebot (today mostly
        // FCPs are missing).

        let section = if has_tags(o, &["C-future-compatibility"]) {
            "Compatibility Notes"
        } else if has_tags(o, &["T-libs", "T-libs-api"]) {
            "Library"
        } else if has_tags(o, &["T-lang"]) {
            "Language"
        } else if has_tags(o, &["T-compiler"]) {
            "Compiler"
        } else {
            "Other"
        };
        by_section.get_mut(section).unwrap().push_str(&format!(
            "- [{title}]({url}/)\n",
            title = o["title"].as_str().unwrap(),
            url = o["url"].as_str().unwrap(),
        ));
    }
}

fn has_tags<'a>(o: &'a json::Value, tags: &[&str]) -> bool {
    o["labels"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|o| tags.iter().any(|tag| o["name"] == *tag))
}

struct Sections {
    language_relnotes: String,
    compiler_relnotes: String,
    libraries_relnotes: String,
    stabilized_apis_relnotes: String,
    const_stabilized_apis_relnotes: String,
    rustdoc_relnotes: String,
    compat_relnotes: String,
    internal_changes_relnotes: String,
    other_relnotes: String,
}

fn to_sections<'a>(
    iter: impl IntoIterator<Item = &'a json::Value>,
    mut tracking: &mut TrackingIssues,
) -> Sections {
    let mut by_section = HashMap::new();
    by_section.insert("Language", String::new());
    by_section.insert("Compiler", String::new());
    by_section.insert("Libraries", String::new());
    by_section.insert("Stabilized APIs", String::new());
    by_section.insert("Const Stabilized APIs", String::new());
    by_section.insert("Rustdoc", String::new());
    by_section.insert("Compatibility Notes", String::new());
    by_section.insert("Internal Changes", String::new());
    by_section.insert("Other", String::new());

    map_to_line_items(iter, &mut tracking, &mut by_section);

    Sections {
        language_relnotes: by_section.remove("Language").unwrap(),
        compiler_relnotes: by_section.remove("Compiler").unwrap(),
        libraries_relnotes: by_section.remove("Libraries").unwrap(),
        stabilized_apis_relnotes: by_section.remove("Stabilized APIs").unwrap(),
        const_stabilized_apis_relnotes: by_section.remove("Const Stabilized APIs").unwrap(),
        rustdoc_relnotes: by_section.remove("Rustdoc").unwrap(),
        compat_relnotes: by_section.remove("Compatibility Notes").unwrap(),
        internal_changes_relnotes: by_section.remove("Internal Changes").unwrap(),
        other_relnotes: by_section.remove("Other").unwrap(),
    }
}

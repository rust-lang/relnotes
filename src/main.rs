use std::collections::BTreeMap;
use std::env;

use askama::Template;
use chrono::prelude::*;
use chrono::Duration;

use reqwest::header::HeaderMap;
use serde_json as json;

type JsonRefArray<'a> = Vec<&'a json::Value>;

const SKIP_LABELS: &[&str] = &[
    "beta-nominated",
    "beta-accepted",
    "stable-nominated",
    "stable-accepted",
    "rollup",
];

#[derive(Clone, Template)]
#[template(path = "relnotes.md", escape = "none")]
struct ReleaseNotes {
    cargo_links: String,
    cargo_relnotes: String,
    cargo_unsorted: String,
    compat_relnotes: String,
    compat_unsorted: String,
    compiler_relnotes: String,
    compiler_unsorted: String,
    date: NaiveDate,
    language_relnotes: String,
    language_unsorted: String,
    libraries_relnotes: String,
    libraries_unsorted: String,
    links: String,
    unsorted: String,
    unsorted_relnotes: String,
    version: String,
}

fn main() {
    let mut args = env::args();
    let _ = args.next();
    let version = args
        .next()
        .expect("A version number (xx.yy.zz) for the Rust release is required.");
    let today = Utc::now().date();

    // A known rust release date. (1.42.0)
    let mut end = Utc.ymd(2020, 3, 12);
    let six_weeks = Duration::weeks(6);

    // Get the most recent rust release date.
    while today - end > six_weeks {
        end = end + six_weeks;
    }

    let mut issues = get_issues_by_milestone(&version, "rust");
    issues.sort_by_cached_key(|issue| issue["number"].as_u64().unwrap());

    // Skips `beta-accepted` as those PRs were backported onto the
    // previous stable.
    let in_release = issues.iter().filter(|v| {
        !v["labels"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| SKIP_LABELS.contains(&o["name"].as_str().unwrap()))
    });

    let relnotes_tags = &["relnotes", "finished-final-comment-period", "needs-fcp"];

    let links = map_to_link_items("", in_release.clone());
    let (relnotes, rest) = partition_by_tag(in_release, relnotes_tags);

    let (
        compat_relnotes,
        libraries_relnotes,
        language_relnotes,
        compiler_relnotes,
        unsorted_relnotes,
    ) = partition_prs(relnotes);

    let (compat_unsorted, libraries_unsorted, language_unsorted, compiler_unsorted, unsorted) =
        partition_prs(rest);

    let mut cargo_issues = get_issues_by_milestone(&version, "cargo");
    cargo_issues.sort_by_cached_key(|issue| issue["number"].as_u64().unwrap());

    let (cargo_relnotes, cargo_unsorted) = {
        let (relnotes, rest) = partition_by_tag(cargo_issues.iter(), relnotes_tags);

        (
            map_to_line_items("cargo/", relnotes),
            map_to_line_items("cargo/", rest),
        )
    };

    let cargo_links = map_to_link_items("cargo/", cargo_issues.iter());

    let relnotes = ReleaseNotes {
        version,
        date: (end + six_weeks).naive_utc(),
        compat_relnotes,
        compat_unsorted,
        language_relnotes,
        language_unsorted,
        libraries_relnotes,
        libraries_unsorted,
        compiler_relnotes,
        compiler_unsorted,
        cargo_relnotes,
        cargo_unsorted,
        unsorted_relnotes,
        unsorted,
        links,
        cargo_links,
    };

    println!("{}", relnotes.render().unwrap());
}

fn get_issues_by_milestone(version: &str, repo_name: &'static str) -> Vec<json::Value> {
    use reqwest::blocking::Client;

    let headers = request_header();
    let mut args = BTreeMap::new();
    args.insert("states", String::from("[MERGED]"));
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
                            pullRequests({args}) {{
                                nodes {{
                                    number
                                    title
                                    url
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

        let json = client
            .post("https://api.github.com/graphql")
            .headers(headers.clone())
            .body(json_query)
            .send()
            .unwrap()
            .json::<json::Value>()
            .unwrap();

        let milestones_data = json["data"]["repository"]["milestones"].clone();
        assert_eq!(
            milestones_data["totalCount"].as_u64().unwrap(),
            1,
            "More than one milestone matched the query \"{version}\". Please be more specific.",
            version = version
        );
        let pull_requests_data = milestones_data["nodes"][0]["pullRequests"].clone();

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

fn map_to_line_items<'a>(
    prefix: &'static str,
    iter: impl IntoIterator<Item = &'a json::Value>,
) -> String {
    iter.into_iter()
        .map(|o| {
            format!(
                "- [{title}][{prefix}{number}]",
                prefix = prefix,
                title = o["title"].as_str().unwrap(),
                number = o["number"],
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn map_to_link_items<'a>(
    prefix: &'static str,
    iter: impl IntoIterator<Item = &'a json::Value>,
) -> String {
    iter.into_iter()
        .map(|o| {
            format!(
                "[{prefix}{number}]: {url}/",
                prefix = prefix,
                number = o["number"],
                url = o["url"].as_str().unwrap()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn partition_by_tag<'a>(
    iter: impl IntoIterator<Item = &'a json::Value>,
    tags: &[&str],
) -> (JsonRefArray<'a>, JsonRefArray<'a>) {
    iter.into_iter().partition(|o| {
        o["labels"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| tags.iter().any(|tag| o["name"] == *tag))
    })
}

fn partition_prs<'a>(
    iter: impl IntoIterator<Item = &'a json::Value>,
) -> (String, String, String, String, String) {
    let (compat_notes, rest) = partition_by_tag(iter, &["C-future-compatibility"]);
    let (libs, rest) = partition_by_tag(rest, &["T-libs", "T-libs-api"]);
    let (lang, rest) = partition_by_tag(rest, &["T-lang"]);
    let (compiler, rest) = partition_by_tag(rest, &["T-compiler"]);

    (
        map_to_line_items("", compat_notes),
        map_to_line_items("", libs),
        map_to_line_items("", lang),
        map_to_line_items("", compiler),
        map_to_line_items("", rest),
    )
}

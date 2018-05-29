#!/usr/bin/env perl6

use v6;
use HTTP::UserAgent;
use URI::Escape;
use JSON::Fast;

my $current = Date.today;
my $end = Date.new(2017, 7, 20);
until ($current - $end) < (6 * 7) {
    $end = $end.later(:6weeks);
}

my $start = $end.earlier(:6weeks);


sub get-issues($repo-name) {
    my $found_all = False;
    my $uri = 'https://api.github.com/graphql';
    my %headers =
        Content-Type => 'application/json',
        Accept => 'application/json',
        Authorization => "Bearer {%*ENV<GITHUB_API_KEY>}",
        User-Agent => 'Rsearcher/0.0.1';

    my @issues = [];

    my %args =
        states => '[MERGED]',
        last => 100;

    while True {
        my $args = %args.kv.map(-> $k, $v {"$k: $v"}).join(",");
        my $query = q:s:to/END/.trans(' ' => '').trans(["\n", '"'] => [' ', '\"']);
        query {
            repository(owner: "rust-lang", name: "$repo-name") {
                pullRequests($args) {
                    nodes {
                        mergedAt
                        number
                        title
                        url
                        labels(last: 100) {
                            nodes {
                                name
                            }
                        }
                    }
                    pageInfo {
                        startCursor
                    }
                }
            }
        }
        END

        my $json = qq:!c[{"query":"$query"}];
        my $ua = HTTP::UserAgent.new;
        my $request = HTTP::Request.new(POST => $uri, |%headers);

        $request.add-content($json);

        my $response = from-json($ua.request($request).content);
        my $data = $response<data><repository><pullRequests>;
        my @results = $data<nodes>.grep({
            $end > DateTime.new($_<mergedAt>).Date > $start
        });

        if @results.Bool {
            @issues.append(@results);
            %args<before> = qq["$data<pageInfo><startCursor>"];
        } else {
            last;
        }
    }

    @issues
}
my @issues = get-issues('rust');
# find the previous date of release and the next date.
my (@, @no_beta) := @issues.classify({
  $_<labels><nodes>.contains({name => 'beta-accepted'})
}){True, False};

my (@, @no_docs) := @no_beta.classify({
  $_<labels><nodes>.contains({name => 'T-doc'})
}){True, False};

my $links = @no_docs.map({"[$_<number>]: $_<url>"}).join("\n");

my (@libraries, @non_libraries) := @no_docs.classify({
  $_<labels><nodes>.contains({name => 'T-libs'})
}){True, False};

my (:@compiler, :@no_compiler) := @non_libraries.classify({
  $_<labels><nodes>.contains({name => 'T-compiler'}) ?? <compiler> !! <no_compiler>
});

my (@languages, @non_languages) := @no_compiler.classify({
  $_<labels><nodes>.contains({name => 'T-lang'})
}){True, False};

my (@relnotes, @unsorted) := @non_languages.classify({
  $_<labels><nodes>.contains({name => 'relnotes'})
}){True, False};

my (@cargo, @) := get-issues('cargo').classify({
    $_<labels><nodes>.contains({name => 'relnotes'})
}){True, False};

my &map_to_line_item = {"- [$_<title>][$_<number>]"};

my $relnotes = @relnotes.map(&map_to_line_item).join("\n");
my $libraries = @libraries.map(&map_to_line_item).join("\n");
my $compiler = @compiler.map(&map_to_line_item).join("\n");
my $languages = @languages.map(&map_to_line_item).join("\n");
my $unsorted = @unsorted.map(&map_to_line_item).join("\n");
my $cargo = @cargo.map({"- [$_<title>][cargo/$_<number>]"}).join("\n");
my $cargo-links = @cargo.map({"[cargo/$_<number>]: $_<url>"}).join("\n");

say qq:to/END/;
Version @*ARGS[0] ({$end.later(:6weeks)})
==========================

Language
--------
$languages

Compiler
--------
$compiler

Libraries
---------
$libraries

Stabilized APIs
---------------

Cargo
-----
$cargo

Misc
----

Compatibility Notes
-------------------

RELNOTES
--------
$relnotes

UNSORTED
--------
$unsorted

$links
$cargo-links

END

#![allow(missing_docs)]

use std::{
    fmt::{self, Display},
    fs::File,
    io::{BufRead, BufReader, Write},
    time::Duration,
};

use serde::{Deserialize, Deserializer};

// Small
const GO_QUERY_MISS_FAST_SMALL: UnitTime = UnitTime(Duration::from_nanos(589));
const GO_QUERY_MISS_SLOW_SMALL: UnitTime = UnitTime(Duration::from_nanos(1617));

const GO_QUERY_HIT_FAST_SMALL: UnitTime = UnitTime(Duration::from_nanos(61));
const GO_QUERY_HIT_SLOW_SMALL: UnitTime = UnitTime(Duration::from_nanos(2960));

const GO_ITER_FAST_SMALL: UnitTime = UnitTime(Duration::from_nanos(505801));
const GO_ITER_SLOW_SMALL: UnitTime = UnitTime(Duration::from_nanos(2181263));

const GO_UPDATE_SMALL: UnitTime = UnitTime(Duration::from_nanos(29918));
const GO_QUERY_BLOCKS_SMALL: UnitTime = UnitTime(Duration::from_nanos(7348834));

// Medium
const GO_QUERY_MISS_FAST_MEDIUM: UnitTime = UnitTime(Duration::from_nanos(2340));
const GO_QUERY_MISS_SLOW_MEDIUM: UnitTime = UnitTime(Duration::from_nanos(9099));

const GO_QUERY_HIT_FAST_MEDIUM: UnitTime = UnitTime(Duration::from_nanos(406));
const GO_QUERY_HIT_SLOW_MEDIUM: UnitTime = UnitTime(Duration::from_nanos(12909));

const GO_ITER_FAST_MEDIUM: UnitTime = UnitTime(Duration::from_nanos(41978635));
const GO_ITER_SLOW_MEDIUM: UnitTime = UnitTime(Duration::from_nanos(964896104));

const GO_UPDATE_MEDIUM: UnitTime = UnitTime(Duration::from_nanos(116014));
const GO_QUERY_BLOCKS_MEDIUM: UnitTime = UnitTime(Duration::from_nanos(16063524));

// Large
const GO_QUERY_MISS_FAST_LARGE: UnitTime = UnitTime(Duration::from_nanos(5139));
const GO_QUERY_MISS_SLOW_LARGE: UnitTime = UnitTime(Duration::from_nanos(17639));

const GO_QUERY_HIT_FAST_LARGE: UnitTime = UnitTime(Duration::from_nanos(5339));
const GO_QUERY_HIT_SLOW_LARGE: UnitTime = UnitTime(Duration::from_nanos(23944));

const GO_ITER_FAST_LARGE: UnitTime = UnitTime(Duration::from_nanos(651533418));
const GO_ITER_SLOW_LARGE: UnitTime = UnitTime(Duration::from_nanos(8784634345));

const GO_UPDATE_LARGE: UnitTime = UnitTime(Duration::from_nanos(242246));
const GO_QUERY_BLOCKS_LARGE: UnitTime = UnitTime(Duration::from_nanos(54795291));

struct GoResult {
    query_miss_fast: UnitTime,
    query_miss_slow: UnitTime,
    query_hit_fast: UnitTime,
    query_hit_slow: UnitTime,
    iter_fast: UnitTime,
    iter_slow: UnitTime,
    update: UnitTime,
    query_blocks: UnitTime,
}

const GO_RESULT_SMALL: GoResult = GoResult {
    query_miss_fast: GO_QUERY_MISS_FAST_SMALL,
    query_miss_slow: GO_QUERY_MISS_SLOW_SMALL,
    query_hit_fast: GO_QUERY_HIT_FAST_SMALL,
    query_hit_slow: GO_QUERY_HIT_SLOW_SMALL,
    iter_fast: GO_ITER_FAST_SMALL,
    iter_slow: GO_ITER_SLOW_SMALL,
    update: GO_UPDATE_SMALL,
    query_blocks: GO_QUERY_BLOCKS_SMALL,
};

const GO_RESULT_MEDIUM: GoResult = GoResult {
    query_miss_fast: GO_QUERY_MISS_FAST_MEDIUM,
    query_miss_slow: GO_QUERY_MISS_SLOW_MEDIUM,
    query_hit_fast: GO_QUERY_HIT_FAST_MEDIUM,
    query_hit_slow: GO_QUERY_HIT_SLOW_MEDIUM,
    iter_fast: GO_ITER_FAST_MEDIUM,
    iter_slow: GO_ITER_SLOW_MEDIUM,
    update: GO_UPDATE_MEDIUM,
    query_blocks: GO_QUERY_BLOCKS_MEDIUM,
};

const GO_RESULT_LARGE: GoResult = GoResult {
    query_miss_fast: GO_QUERY_MISS_FAST_LARGE,
    query_miss_slow: GO_QUERY_MISS_SLOW_LARGE,
    query_hit_fast: GO_QUERY_HIT_FAST_LARGE,
    query_hit_slow: GO_QUERY_HIT_SLOW_LARGE,
    iter_fast: GO_ITER_FAST_LARGE,
    iter_slow: GO_ITER_SLOW_LARGE,
    update: GO_UPDATE_LARGE,
    query_blocks: GO_QUERY_BLOCKS_LARGE,
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Unit {
    Second,
    MilliSecond,
    MicroSecond,
    NanoSecond,
}

impl<'de> Deserialize<'de> for Unit {
    fn deserialize<D>(deserializer: D) -> Result<Unit, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(UnitVisitor)
    }
}

struct UnitVisitor;

impl<'de> serde::de::Visitor<'de> for UnitVisitor {
    type Value = Unit;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("either ns, µs, ms or s")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if v == "ns" {
            Ok(Unit::NanoSecond)
        } else if v == "µs" {
            Ok(Unit::MicroSecond)
        } else if v == "ms" {
            Ok(Unit::MilliSecond)
        } else if v == "s" {
            Ok(Unit::Second)
        } else {
            Err(E::custom(format!("unit {} not recognized", v)))
        }
    }
}

#[derive(Deserialize, Debug)]
struct Measurement {
    estimate: f64,
    unit: Unit,
}

#[derive(Deserialize, Debug)]
struct InputBenchResult {
    id: String,
    mean: Measurement,
}

/// Script to generate markdown table with benchmark results.
/// NOTE: This doesn't belong in the examples directory but I couldn't
/// find a better place for it.
fn main() {
    let file = File::open("benchmark.json").expect("failed to open file");
    let reader = BufReader::new(file);

    let mut full_results = FullResults::default();

    for line in reader.lines() {
        let bench: Result<InputBenchResult, _> =
            serde_json::from_str(&line.expect("failed to read line"));

        if let Ok(bench) = bench {
            let time = bench.mean.into();

            if bench.id.starts_with("query-miss/Params { _name: \"small") {
                let ratio = BenchRatio::new(&time, &GO_QUERY_MISS_SLOW_SMALL);
                full_results.small.query_miss = BenchResult { time, ratio };
            } else if bench.id.starts_with("query-miss/Params { _name: \"medium") {
                let ratio = BenchRatio::new(&time, &GO_QUERY_MISS_SLOW_MEDIUM);
                full_results.medium.query_miss = BenchResult { time, ratio };
            } else if bench.id.starts_with("query-miss/Params { _name: \"large") {
                let ratio = BenchRatio::new(&time, &GO_QUERY_MISS_SLOW_LARGE);
                full_results.large.query_miss = BenchResult { time, ratio };
            } else if bench.id.starts_with("query-hits/Params { _name: \"small") {
                let ratio = BenchRatio::new(&time, &GO_QUERY_HIT_SLOW_SMALL);
                full_results.small.query_hit = BenchResult { time, ratio };
            } else if bench.id.starts_with("query-hits/Params { _name: \"medium") {
                let ratio = BenchRatio::new(&time, &GO_QUERY_HIT_SLOW_MEDIUM);
                full_results.medium.query_hit = BenchResult { time, ratio };
            } else if bench.id.starts_with("query-hits/Params { _name: \"large") {
                let ratio = BenchRatio::new(&time, &GO_QUERY_HIT_SLOW_LARGE);
                full_results.large.query_hit = BenchResult { time, ratio };
            } else if bench.id.starts_with("iavl-range/Params { _name: \"small") {
                let ratio = BenchRatio::new(&time, &GO_ITER_SLOW_SMALL);
                full_results.small.iter = BenchResult { time, ratio };
            } else if bench.id.starts_with("iavl-range/Params { _name: \"medium") {
                let ratio = BenchRatio::new(&time, &GO_ITER_SLOW_MEDIUM);
                full_results.medium.iter = BenchResult { time, ratio };
            } else if bench.id.starts_with("iavl-range/Params { _name: \"large") {
                let ratio = BenchRatio::new(&time, &GO_ITER_SLOW_LARGE);
                full_results.large.iter = BenchResult { time, ratio };
            } else if bench.id.starts_with("iavl-update/Params { _name: \"small") {
                let ratio = BenchRatio::new(&time, &GO_UPDATE_SMALL);
                full_results.small.update = BenchResult { time, ratio };
            } else if bench.id.starts_with("iavl-update/Params { _name: \"medium") {
                let ratio = BenchRatio::new(&time, &GO_UPDATE_MEDIUM);
                full_results.medium.update = BenchResult { time, ratio };
            } else if bench.id.starts_with("iavl-update/Params { _name: \"large") {
                let ratio = BenchRatio::new(&time, &GO_UPDATE_LARGE);
                full_results.large.update = BenchResult { time, ratio };
            } else if bench
                .id
                .starts_with("iavl-run-blocks/Params { _name: \"small")
            {
                let ratio = BenchRatio::new(&time, &GO_QUERY_BLOCKS_SMALL);
                full_results.small.run_blocks = BenchResult { time, ratio };
            } else if bench
                .id
                .starts_with("iavl-run-blocks/Params { _name: \"medium")
            {
                let ratio = BenchRatio::new(&time, &GO_QUERY_BLOCKS_MEDIUM);
                full_results.medium.run_blocks = BenchResult { time, ratio };
            } else if bench
                .id
                .starts_with("iavl-run-blocks/Params { _name: \"large")
            {
                let ratio = BenchRatio::new(&time, &GO_QUERY_BLOCKS_LARGE);
                full_results.large.run_blocks = BenchResult { time, ratio };
            };
        }
    }

    let mut handlebars = handlebars::Handlebars::new();
    handlebars
        .register_template_string("bench_small", get_bench_template(GO_RESULT_SMALL))
        .expect("hard coded config template is valid");
    handlebars
        .register_template_string("bench_medium", get_bench_template(GO_RESULT_MEDIUM))
        .expect("hard coded config template is valid");
    handlebars
        .register_template_string("bench_large", get_bench_template(GO_RESULT_LARGE))
        .expect("hard coded config template is valid");

    let small_table = handlebars
        .render("bench_small", &full_results.small)
        .expect("OutputResult will always work with the BENCH_TEMPLATE");
    let medium_table = handlebars
        .render("bench_medium", &full_results.medium)
        .expect("OutputResult will always work with the BENCH_TEMPLATE");
    let large_table = handlebars
        .render("bench_large", &full_results.large)
        .expect("OutputResult will always work with the BENCH_TEMPLATE");

    let mut file = std::fs::File::create("benchmark.md").expect("failed to create a new file");
    file.write("# Benchmark\n".as_bytes())
        .expect("failed to write");
    file.write("## Small".as_bytes()).expect("failed to write");
    file.write_all(small_table.as_bytes())
        .expect("failed to write");
    file.write("## Medium".as_bytes()).expect("failed to write");
    file.write_all(medium_table.as_bytes())
        .expect("failed to write");
    file.write("## Large".as_bytes()).expect("failed to write");
    file.write_all(large_table.as_bytes())
        .expect("failed to write");
}

#[derive(serde::Serialize, Default)]
struct BenchResult {
    time: UnitTime,
    ratio: BenchRatio,
}

struct BenchRatio(f64);

impl BenchRatio {
    fn new(gears_time: &UnitTime, go_time: &UnitTime) -> Self {
        BenchRatio(gears_time.to_seconds() / go_time.to_seconds())
    }
}

impl Default for BenchRatio {
    fn default() -> Self {
        Self(f64::INFINITY)
    }
}

impl serde::Serialize for BenchRatio {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = if self.0 == f64::INFINITY {
            "<mark style=\"background-color: red\">&nbsp;NOT RUN&nbsp;</mark>".to_string()
        } else if self.0 > 1.0 {
            format!(
                "<mark style=\"background-color: red\">&nbsp;{:.1}&nbsp;</mark>",
                self.0
            )
        } else {
            format!(
                "<mark style=\"background-color: green\">&nbsp;{:.1}&nbsp;</mark>",
                self.0
            )
        };

        serializer.collect_str(&s)
    }
}

/// Wrapper around Duration to allow a different Serialize implementation
#[derive(Debug, Clone, PartialEq, Default)]
struct UnitTime(Duration);

impl From<Measurement> for UnitTime {
    fn from(value: Measurement) -> Self {
        match value.unit {
            Unit::Second => UnitTime(Duration::from_secs(value.estimate as u64)),
            Unit::MilliSecond => UnitTime(Duration::from_millis(value.estimate as u64)),
            Unit::MicroSecond => UnitTime(Duration::from_micros(value.estimate as u64)),
            Unit::NanoSecond => UnitTime(Duration::from_nanos(value.estimate as u64)),
        }
    }
}

impl UnitTime {
    fn to_seconds(&self) -> f64 {
        self.0.as_secs_f64()
    }
}

impl Display for UnitTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl serde::Serialize for UnitTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(&self.to_string())
    }
}

#[derive(serde::Serialize, Default)]
struct TemplatedResults {
    query_miss: BenchResult,
    query_hit: BenchResult,
    iter: BenchResult,
    update: BenchResult,
    run_blocks: BenchResult,
}

#[derive(Default)]

struct FullResults {
    small: TemplatedResults,
    medium: TemplatedResults,
    large: TemplatedResults,
}

fn get_bench_template(go_results: GoResult) -> String {
    format!(
        r#"
| Test              | Gears                      | Go           | Ratio                               |
| :---------------- | :------------------------- | :----------  | :---------------------------------- |
| Query miss (fast) |                            | {}           |      |
| Query miss (slow) |  {{{{ query_miss.time }}}} | {}           | {{{{{{ query_miss.ratio }}}}}}                                     |
| Query hit (fast)  |                            | {}           |       |
| Query hit (slow)  |  {{{{ query_hit.time }}}}  | {}           | {{{{{{ query_hit.ratio }}}}}}                                     |
| Iter (fast)       |                            | {}           |            |
| Iter (slow)       | {{{{ iter.time }}}}        | {}           | {{{{{{ iter.ratio }}}}}}                                     |
| Update            |  {{{{ update.time }}}}     | {}           | {{{{{{ update.ratio }}}}}}          |
| Run Blocks        |  {{{{ run_blocks.time }}}} | {}           | {{{{{{ run_blocks.ratio }}}}}}      |
"#,
        go_results.query_miss_fast,
        go_results.query_miss_slow,
        go_results.query_hit_fast,
        go_results.query_hit_slow,
        go_results.iter_fast,
        go_results.iter_slow,
        go_results.update,
        go_results.query_blocks,
    )
}

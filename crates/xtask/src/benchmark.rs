use anyhow::{Context, Result, bail};
use roxmltree::{Document, Node};
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const DEFAULT_DURATION_SECS: u64 = 12;
const TRACE_PADDING_SECS: u64 = 2;

pub(crate) fn run(mut args: impl Iterator<Item = String>) -> Result<()> {
    let Some(command) = args.next() else {
        bail!(
            "usage: cargo run -p xtask -- <benchmark-driver|benchmark-compare> [options]"
        );
    };

    match command.as_str() {
        "benchmark-driver" => run_driver(args),
        "benchmark-compare" => run_compare(args),
        other => bail!("unknown benchmark command `{other}`"),
    }
}

fn run_driver(mut args: impl Iterator<Item = String>) -> Result<()> {
    let mut scenario = None;
    let mut duration_secs = DEFAULT_DURATION_SECS;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--scenario" => {
                let value = args.next().context("missing value for --scenario")?;
                scenario = Some(Scenario::parse(&value)?);
            }
            "--duration-secs" => {
                let value = args.next().context("missing value for --duration-secs")?;
                duration_secs = value
                    .parse()
                    .with_context(|| format!("invalid --duration-secs `{value}`"))?;
            }
            other => bail!(
                "unknown benchmark-driver argument `{other}`; expected --scenario or --duration-secs"
            ),
        }
    }

    let scenario = scenario.context("missing required --scenario")?;
    scenario.run(Duration::from_secs(duration_secs))
}

fn run_compare(mut args: impl Iterator<Item = String>) -> Result<()> {
    let mut baseline_root = None;
    let mut candidate_root = None;
    let mut output_root = None;
    let mut duration_secs = DEFAULT_DURATION_SECS;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--baseline-root" => {
                baseline_root = Some(PathBuf::from(
                    args.next().context("missing value for --baseline-root")?,
                ));
            }
            "--candidate-root" => {
                candidate_root = Some(PathBuf::from(
                    args.next().context("missing value for --candidate-root")?,
                ));
            }
            "--output" => {
                output_root = Some(PathBuf::from(
                    args.next().context("missing value for --output")?,
                ));
            }
            "--duration-secs" => {
                let value = args.next().context("missing value for --duration-secs")?;
                duration_secs = value
                    .parse()
                    .with_context(|| format!("invalid --duration-secs `{value}`"))?;
            }
            other => bail!(
                "unknown benchmark-compare argument `{other}`; expected --baseline-root, --candidate-root, --output, or --duration-secs"
            ),
        }
    }

    let baseline_root = canonicalize_root(baseline_root.context("missing --baseline-root")?)?;
    let candidate_root = canonicalize_root(candidate_root.context("missing --candidate-root")?)?;
    let output_root = output_root.context("missing --output")?;
    fs::create_dir_all(&output_root)
        .with_context(|| format!("failed to create {}", output_root.display()))?;

    let baseline = BuildSpec::new("baseline", baseline_root)?;
    let candidate = BuildSpec::new("candidate", candidate_root)?;

    build_release_binaries(&baseline)?;
    build_release_binaries(&candidate)?;

    let scenarios = Scenario::all();
    let mut runs = Vec::with_capacity(scenarios.len() * 2);
    for build in [&baseline, &candidate] {
        for scenario in scenarios {
            runs.push(run_single_benchmark(
                build,
                *scenario,
                duration_secs,
                &output_root,
            )?);
        }
    }

    let summary = ComparisonSummary::from_runs(&baseline, &candidate, runs)?;
    write_report_artifacts(&output_root, &summary)?;
    println!("wrote benchmark report to {}", output_root.display());
    Ok(())
}

fn canonicalize_root(path: PathBuf) -> Result<PathBuf> {
    path.canonicalize()
        .with_context(|| format!("failed to canonicalize {}", path.display()))
}

#[derive(Clone, Debug)]
struct BuildSpec {
    label: &'static str,
    root: PathBuf,
    git_sha: String,
}

impl BuildSpec {
    fn new(label: &'static str, root: PathBuf) -> Result<Self> {
        Ok(Self {
            label,
            git_sha: git_rev_parse_short(&root)?,
            root,
        })
    }

    fn termy_binary(&self) -> PathBuf {
        self.root.join("target/release/termy")
    }

    fn xtask_binary(&self) -> PathBuf {
        self.root.join("target/release/xtask")
    }
}

fn git_rev_parse_short(root: &Path) -> Result<String> {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--short")
        .arg("HEAD")
        .current_dir(root)
        .output()
        .with_context(|| format!("failed to run git rev-parse in {}", root.display()))?;
    if !output.status.success() {
        bail!(
            "git rev-parse failed in {}: {}",
            root.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn build_release_binaries(build: &BuildSpec) -> Result<()> {
    run_command(
        Command::new("cargo")
            .arg("build")
            .arg("--release")
            .arg("-p")
            .arg("termy")
            .arg("-p")
            .arg("xtask")
            .current_dir(&build.root),
        format!("cargo build --release in {}", build.root.display()),
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Scenario {
    BurstOutput,
    SteadyScroll,
    AltScreenAnim,
}

impl Scenario {
    fn all() -> &'static [Scenario] {
        &[
            Scenario::BurstOutput,
            Scenario::SteadyScroll,
            Scenario::AltScreenAnim,
        ]
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "burst-output" => Ok(Self::BurstOutput),
            "steady-scroll" => Ok(Self::SteadyScroll),
            "alt-screen-anim" => Ok(Self::AltScreenAnim),
            other => bail!("unknown benchmark scenario `{other}`"),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::BurstOutput => "burst-output",
            Self::SteadyScroll => "steady-scroll",
            Self::AltScreenAnim => "alt-screen-anim",
        }
    }

    fn run(self, duration: Duration) -> Result<()> {
        match self {
            Self::BurstOutput => run_burst_output(duration),
            Self::SteadyScroll => run_steady_scroll(duration),
            Self::AltScreenAnim => run_alt_screen_anim(duration),
        }
    }
}

fn run_burst_output(duration: Duration) -> Result<()> {
    let start = Instant::now();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut burst = 0u64;
    while start.elapsed() < duration {
        for line in 0..12u64 {
            writeln!(
                out,
                "burst {burst:05}  line {line:02} 0123456789 abcdefghijklmnopqrstuvwxyz"
            )?;
        }
        out.flush()?;
        burst = burst.saturating_add(1);
        thread::sleep(Duration::from_millis(28));
    }
    Ok(())
}

fn run_steady_scroll(duration: Duration) -> Result<()> {
    let start = Instant::now();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut line = 0u64;
    while start.elapsed() < duration {
        writeln!(
            out,
            "scroll {line:06}  the quick brown fox jumps over the lazy dog 0123456789"
        )?;
        out.flush()?;
        line = line.saturating_add(1);
        thread::sleep(Duration::from_millis(6));
    }
    Ok(())
}

fn run_alt_screen_anim(duration: Duration) -> Result<()> {
    let start = Instant::now();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    write!(out, "\x1b[?1049h\x1b[?25l")?;
    out.flush()?;

    let rows = 24usize;
    let cols = 80usize;
    let mut frame = 0usize;
    while start.elapsed() < duration {
        write!(out, "\x1b[H")?;
        for row in 0..rows {
            let band = (row + frame) % cols;
            let mut line = String::with_capacity(cols);
            for col in 0..cols {
                let ch = if col == band || col == (band + 1) % cols {
                    '#'
                } else if (col + row + frame) % 7 == 0 {
                    '.'
                } else {
                    ' '
                };
                line.push(ch);
            }
            writeln!(out, "{line}")?;
        }
        write!(
            out,
            "\x1b[2;2Hframe {:05} elapsed {:.2}s\x1b[{};{}H",
            frame,
            start.elapsed().as_secs_f32(),
            (frame % rows) + 1,
            ((frame * 3) % cols) + 1
        )?;
        out.flush()?;
        frame = frame.saturating_add(1);
        thread::sleep(Duration::from_millis(16));
    }

    write!(out, "\x1b[?25h\x1b[?1049l")?;
    out.flush()?;
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AppSummary {
    build_label: Option<String>,
    git_sha: Option<String>,
    scenario: String,
    duration_ms: u64,
    sample_count: u64,
    total_frames: u64,
    fps_avg: f32,
    frame_p50_ms: f32,
    frame_p95_ms: f32,
    frame_p99_ms: f32,
    cpu_avg_percent: f32,
    cpu_max_percent: f32,
    memory_max_bytes: u64,
    runtime_wakeups: u64,
    view_wake_signals: u64,
    terminal_event_drain_passes: u64,
    terminal_redraws: u64,
    alt_screen_fallback_redraws: u64,
    grid_paint_count: u64,
    shape_line_calls: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct EnergySummary {
    trace_template: String,
    cpu_total_ns: Option<u64>,
    cpu_percent: Option<f32>,
    idle_wakeups: Option<u64>,
    memory_bytes: Option<u64>,
    disk_bytes_read: Option<u64>,
    disk_bytes_written: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
struct RunResult {
    build_label: String,
    git_sha: String,
    scenario: String,
    app_summary: AppSummary,
    energy_summary: EnergySummary,
}

fn run_single_benchmark(
    build: &BuildSpec,
    scenario: Scenario,
    duration_secs: u64,
    output_root: &Path,
) -> Result<RunResult> {
    let raw_dir = output_root.join("raw").join(build.label).join(scenario.as_str());
    let energy_dir = output_root.join("energy").join(build.label).join(scenario.as_str());
    let config_root = raw_dir.join("config");
    let metrics_dir = raw_dir.join("app");
    fs::create_dir_all(config_root.join("termy"))
        .with_context(|| format!("failed to create {}", config_root.display()))?;
    fs::create_dir_all(&metrics_dir)
        .with_context(|| format!("failed to create {}", metrics_dir.display()))?;
    fs::create_dir_all(&energy_dir)
        .with_context(|| format!("failed to create {}", energy_dir.display()))?;

    let config_path = config_root.join("termy/config.txt");
    fs::write(&config_path, benchmark_config_contents())
        .with_context(|| format!("failed to write {}", config_path.display()))?;

    let trace_path = energy_dir.join("activity-monitor.trace");
    let command = benchmark_driver_command(build, scenario, duration_secs);
    let time_limit_secs = duration_secs.saturating_add(TRACE_PADDING_SECS);
    let mut activity_command = activity_monitor_command(
        build,
        &trace_path,
        &config_root,
        &metrics_dir,
        scenario,
        &command,
        time_limit_secs,
    );
    run_command(
        &mut activity_command,
        format!("xctrace benchmark run for {} {}", build.label, scenario.as_str()),
    )?;

    let summary_path = metrics_dir.join("summary.json");
    let summary: AppSummary = read_json(&summary_path)?;

    let toc_path = energy_dir.join("toc.xml");
    let live_path = energy_dir.join("activity-monitor-process-live.xml");
    let ledger_path = energy_dir.join("activity-monitor-process-ledger.xml");
    export_xctrace_table(&trace_path, None, &toc_path)?;
    export_xctrace_table(
        &trace_path,
        Some("/trace-toc/run[@number=\"1\"]/data/table[@schema=\"activity-monitor-process-live\"]"),
        &live_path,
    )?;
    export_xctrace_table(
        &trace_path,
        Some(
            "/trace-toc/run[@number=\"1\"]/data/table[@schema=\"activity-monitor-process-ledger\"]",
        ),
        &ledger_path,
    )?;

    let energy_summary = parse_activity_monitor_summary(&live_path, &ledger_path)?;
    let energy_json_path = energy_dir.join("energy.json");
    write_json(&energy_json_path, &energy_summary)?;

    Ok(RunResult {
        build_label: build.label.to_string(),
        git_sha: build.git_sha.clone(),
        scenario: scenario.as_str().to_string(),
        app_summary: summary,
        energy_summary,
    })
}

fn benchmark_config_contents() -> &'static str {
    "tmux_enabled = false\nbackground_blur = false\nbackground_opacity = 1.0\ncursor_blink = false\nwindow_width = 1280\nwindow_height = 820\nshow_debug_overlay = false\n"
}

fn benchmark_driver_command(build: &BuildSpec, scenario: Scenario, duration_secs: u64) -> String {
    format!(
        "{} benchmark-driver --scenario {} --duration-secs {}",
        shell_escape_path(&build.xtask_binary()),
        scenario.as_str(),
        duration_secs
    )
}

fn activity_monitor_command(
    build: &BuildSpec,
    trace_path: &Path,
    config_root: &Path,
    metrics_dir: &Path,
    scenario: Scenario,
    benchmark_command: &str,
    time_limit_secs: u64,
) -> Command {
    let mut command = Command::new("xctrace");
    command
        .arg("record")
        .arg("--template")
        .arg("Activity Monitor")
        .arg("--time-limit")
        .arg(format!("{time_limit_secs}s"))
        .arg("--output")
        .arg(trace_path)
        .arg("--env")
        .arg(format!("XDG_CONFIG_HOME={}", config_root.display()))
        .arg("--env")
        .arg(format!("TERMY_BENCHMARK_COMMAND={benchmark_command}"))
        .arg("--env")
        .arg(format!("TERMY_BENCHMARK_SCENARIO={}", scenario.as_str()))
        .arg("--env")
        .arg(format!("TERMY_BENCHMARK_METRICS_PATH={}", metrics_dir.display()))
        .arg("--env")
        .arg("TERMY_BENCHMARK_EXIT_ON_COMPLETE=1")
        .arg("--env")
        .arg(format!("TERMY_BENCHMARK_BUILD_LABEL={}", build.label))
        .arg("--env")
        .arg(format!("TERMY_BENCHMARK_GIT_SHA={}", build.git_sha))
        .arg("--launch")
        .arg("--")
        .arg(build.termy_binary());
    command
}

fn export_xctrace_table(trace_path: &Path, xpath: Option<&str>, output_path: &Path) -> Result<()> {
    let mut command = Command::new("xctrace");
    command.arg("export").arg("--input").arg(trace_path);
    if let Some(xpath) = xpath {
        command.arg("--xpath").arg(xpath);
    } else {
        command.arg("--toc");
    }
    command.arg("--output").arg(output_path);
    run_command(
        &mut command,
        format!("xctrace export {}", output_path.display()),
    )
}

fn parse_activity_monitor_summary(live_path: &Path, ledger_path: &Path) -> Result<EnergySummary> {
    let live_xml = fs::read_to_string(live_path)
        .with_context(|| format!("failed to read {}", live_path.display()))?;
    let ledger_xml = fs::read_to_string(ledger_path)
        .with_context(|| format!("failed to read {}", ledger_path.display()))?;

    let live_row = parse_single_row_table(&live_xml)?;
    let ledger_row = parse_single_row_table(&ledger_xml)?;

    Ok(EnergySummary {
        trace_template: "Activity Monitor".to_string(),
        cpu_total_ns: ledger_row
            .get("cpu-total")
            .and_then(|value| value.parse::<u64>().ok()),
        cpu_percent: live_row
            .get("cpu-percent")
            .and_then(|value| value.parse::<f32>().ok()),
        idle_wakeups: ledger_row
            .get("idle-wakeups")
            .and_then(|value| value.parse::<u64>().ok()),
        memory_bytes: live_row
            .get("memory-physical-footprint")
            .and_then(|value| value.parse::<u64>().ok()),
        disk_bytes_read: ledger_row
            .get("disk-bytes-read")
            .and_then(|value| value.parse::<u64>().ok()),
        disk_bytes_written: ledger_row
            .get("disk-bytes-written")
            .and_then(|value| value.parse::<u64>().ok()),
    })
}

fn parse_single_row_table(xml: &str) -> Result<std::collections::HashMap<String, String>> {
    let doc = Document::parse(xml).context("failed to parse xctrace xml")?;
    let node = doc
        .descendants()
        .find(|node| node.has_tag_name("node"))
        .context("missing trace-query-result node")?;
    let schema = node
        .children()
        .find(|child| child.has_tag_name("schema"))
        .context("missing schema node")?;
    let mnemonics = schema
        .children()
        .filter(|child| child.has_tag_name("col"))
        .map(schema_mnemonic)
        .collect::<Result<Vec<_>>>()?;
    let row = node
        .children()
        .find(|child| child.has_tag_name("row"))
        .context("missing row node")?;

    let mut values = std::collections::HashMap::new();
    for (mnemonic, cell) in mnemonics
        .into_iter()
        .zip(row.children().filter(Node::is_element))
    {
        if cell.has_tag_name("sentinel") {
            continue;
        }
        if let Some(value) = node_value(cell) {
            values.insert(mnemonic, value);
        }
    }
    Ok(values)
}

fn schema_mnemonic(node: Node<'_, '_>) -> Result<String> {
    node.children()
        .find(|child| child.has_tag_name("mnemonic"))
        .and_then(|mnemonic| mnemonic.text())
        .map(ToOwned::to_owned)
        .context("missing schema mnemonic")
}

fn node_value(node: Node<'_, '_>) -> Option<String> {
    node.text()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn shell_escape_path(path: &Path) -> String {
    shell_escape(path.as_os_str())
}

fn shell_escape(value: &OsStr) -> String {
    let value = value.to_string_lossy();
    let escaped = value.replace('\'', "'\"'\"'");
    format!("'{escaped}'")
}

fn run_command(command: &mut Command, description: String) -> Result<()> {
    let status = command
        .stdin(Stdio::null())
        .status()
        .with_context(|| format!("failed to start {description}"))?;
    if !status.success() {
        bail!("{description} failed with status {status}");
    }
    Ok(())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let contents =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&contents).with_context(|| format!("failed to parse {}", path.display()))
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let contents = serde_json::to_string_pretty(value)
        .with_context(|| format!("failed to serialize {}", path.display()))?;
    fs::write(path, contents).with_context(|| format!("failed to write {}", path.display()))
}

#[derive(Clone, Debug, Serialize)]
struct ComparisonSummary {
    baseline_git_sha: String,
    candidate_git_sha: String,
    scenarios: Vec<ScenarioComparison>,
}

impl ComparisonSummary {
    fn from_runs(
        baseline: &BuildSpec,
        candidate: &BuildSpec,
        runs: Vec<RunResult>,
    ) -> Result<Self> {
        let mut scenarios = Vec::new();
        for scenario in Scenario::all() {
            let baseline_run = runs
                .iter()
                .find(|run| run.build_label == baseline.label && run.scenario == scenario.as_str())
                .cloned()
                .with_context(|| format!("missing baseline run for {}", scenario.as_str()))?;
            let candidate_run = runs
                .iter()
                .find(|run| run.build_label == candidate.label && run.scenario == scenario.as_str())
                .cloned()
                .with_context(|| format!("missing candidate run for {}", scenario.as_str()))?;
            scenarios.push(ScenarioComparison::new(
                scenario.as_str().to_string(),
                baseline_run,
                candidate_run,
            ));
        }

        Ok(Self {
            baseline_git_sha: baseline.git_sha.clone(),
            candidate_git_sha: candidate.git_sha.clone(),
            scenarios,
        })
    }
}

#[derive(Clone, Debug, Serialize)]
struct ScenarioComparison {
    scenario: String,
    baseline: RunResult,
    candidate: RunResult,
    deltas: ScenarioDeltas,
}

impl ScenarioComparison {
    fn new(scenario: String, baseline: RunResult, candidate: RunResult) -> Self {
        let deltas = ScenarioDeltas {
            frame_p95_ms: candidate.app_summary.frame_p95_ms - baseline.app_summary.frame_p95_ms,
            frame_p99_ms: candidate.app_summary.frame_p99_ms - baseline.app_summary.frame_p99_ms,
            cpu_avg_percent: candidate.app_summary.cpu_avg_percent
                - baseline.app_summary.cpu_avg_percent,
            idle_wakeups: option_delta(
                candidate.energy_summary.idle_wakeups,
                baseline.energy_summary.idle_wakeups,
            ),
        };
        Self {
            scenario,
            baseline,
            candidate,
            deltas,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct ScenarioDeltas {
    frame_p95_ms: f32,
    frame_p99_ms: f32,
    cpu_avg_percent: f32,
    idle_wakeups: Option<i64>,
}

fn option_delta(candidate: Option<u64>, baseline: Option<u64>) -> Option<i64> {
    match (candidate, baseline) {
        (Some(candidate), Some(baseline)) => Some(candidate as i64 - baseline as i64),
        _ => None,
    }
}

fn write_report_artifacts(output_root: &Path, summary: &ComparisonSummary) -> Result<()> {
    write_json(&output_root.join("summary.json"), summary)?;
    fs::write(output_root.join("report.md"), render_report(summary))
        .with_context(|| format!("failed to write {}", output_root.join("report.md").display()))
}

fn render_report(summary: &ComparisonSummary) -> String {
    let mut report = String::new();
    report.push_str("# Termy Render Benchmark Report\n\n");
    report.push_str(&format!(
        "Baseline `{}` vs candidate `{}`.\n\n",
        summary.baseline_git_sha, summary.candidate_git_sha
    ));

    for scenario in &summary.scenarios {
        report.push_str(&format!("## {}\n\n", scenario.scenario));
        report.push_str("| Metric | Baseline | Candidate | Delta |\n");
        report.push_str("| --- | ---: | ---: | ---: |\n");
        report.push_str(&format!(
            "| Frame p50 ms | {:.2} | {:.2} | {:.2} |\n",
            scenario.baseline.app_summary.frame_p50_ms,
            scenario.candidate.app_summary.frame_p50_ms,
            scenario.candidate.app_summary.frame_p50_ms - scenario.baseline.app_summary.frame_p50_ms
        ));
        report.push_str(&format!(
            "| Frame p95 ms | {:.2} | {:.2} | {:.2} |\n",
            scenario.baseline.app_summary.frame_p95_ms,
            scenario.candidate.app_summary.frame_p95_ms,
            scenario.deltas.frame_p95_ms
        ));
        report.push_str(&format!(
            "| Frame p99 ms | {:.2} | {:.2} | {:.2} |\n",
            scenario.baseline.app_summary.frame_p99_ms,
            scenario.candidate.app_summary.frame_p99_ms,
            scenario.deltas.frame_p99_ms
        ));
        report.push_str(&format!(
            "| FPS avg | {:.2} | {:.2} | {:.2} |\n",
            scenario.baseline.app_summary.fps_avg,
            scenario.candidate.app_summary.fps_avg,
            scenario.candidate.app_summary.fps_avg - scenario.baseline.app_summary.fps_avg
        ));
        report.push_str(&format!(
            "| CPU avg % | {:.2} | {:.2} | {:.2} |\n",
            scenario.baseline.app_summary.cpu_avg_percent,
            scenario.candidate.app_summary.cpu_avg_percent,
            scenario.deltas.cpu_avg_percent
        ));
        report.push_str(&format!(
            "| CPU max % | {:.2} | {:.2} | {:.2} |\n",
            scenario.baseline.app_summary.cpu_max_percent,
            scenario.candidate.app_summary.cpu_max_percent,
            scenario.candidate.app_summary.cpu_max_percent
                - scenario.baseline.app_summary.cpu_max_percent
        ));
        report.push_str(&format!(
            "| Runtime wakeups | {} | {} | {} |\n",
            scenario.baseline.app_summary.runtime_wakeups,
            scenario.candidate.app_summary.runtime_wakeups,
            scenario.candidate.app_summary.runtime_wakeups as i64
                - scenario.baseline.app_summary.runtime_wakeups as i64
        ));
        report.push_str(&format!(
            "| View wake signals | {} | {} | {} |\n",
            scenario.baseline.app_summary.view_wake_signals,
            scenario.candidate.app_summary.view_wake_signals,
            scenario.candidate.app_summary.view_wake_signals as i64
                - scenario.baseline.app_summary.view_wake_signals as i64
        ));
        report.push_str(&format!(
            "| Drain passes | {} | {} | {} |\n",
            scenario.baseline.app_summary.terminal_event_drain_passes,
            scenario.candidate.app_summary.terminal_event_drain_passes,
            scenario.candidate.app_summary.terminal_event_drain_passes as i64
                - scenario.baseline.app_summary.terminal_event_drain_passes as i64
        ));
        report.push_str(&format!(
            "| Redraws | {} | {} | {} |\n",
            scenario.baseline.app_summary.terminal_redraws,
            scenario.candidate.app_summary.terminal_redraws,
            scenario.candidate.app_summary.terminal_redraws as i64
                - scenario.baseline.app_summary.terminal_redraws as i64
        ));
        report.push_str(&format!(
            "| Alt-screen fallback redraws | {} | {} | {} |\n",
            scenario.baseline.app_summary.alt_screen_fallback_redraws,
            scenario.candidate.app_summary.alt_screen_fallback_redraws,
            scenario.candidate.app_summary.alt_screen_fallback_redraws as i64
                - scenario.baseline.app_summary.alt_screen_fallback_redraws as i64
        ));
        report.push_str(&format!(
            "| Idle wakeups | {} | {} | {} |\n\n",
            format_option_u64(scenario.baseline.energy_summary.idle_wakeups),
            format_option_u64(scenario.candidate.energy_summary.idle_wakeups),
            format_option_i64(scenario.deltas.idle_wakeups),
        ));

        report.push_str("Findings:\n");
        report.push_str(&format!(
            "- Candidate {} frame p95 by {:.2} ms.\n",
            if scenario.deltas.frame_p95_ms < 0.0 {
                "improves"
            } else {
                "regresses"
            },
            scenario.deltas.frame_p95_ms.abs()
        ));
        report.push_str(&format!(
            "- Candidate {} CPU avg by {:.2}%.\n\n",
            if scenario.deltas.cpu_avg_percent < 0.0 {
                "reduces"
            } else {
                "increases"
            },
            scenario.deltas.cpu_avg_percent.abs()
        ));
    }

    report
}

fn format_option_u64(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "n/a".to_string())
}

fn format_option_i64(value: Option<i64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "n/a".to_string())
}

#[cfg(test)]
mod tests {
    use super::{Scenario, parse_single_row_table};

    #[test]
    fn parses_scenario_names() {
        assert_eq!(Scenario::parse("burst-output").unwrap(), Scenario::BurstOutput);
        assert!(Scenario::parse("nope").is_err());
    }

    #[test]
    fn parses_single_row_xctrace_table() {
        let xml = r#"<?xml version="1.0"?>
<trace-query-result>
  <node>
    <schema name="example">
      <col><mnemonic>cpu-total</mnemonic></col>
      <col><mnemonic>idle-wakeups</mnemonic></col>
    </schema>
    <row>
      <duration-on-core>42</duration-on-core>
      <event-count>7</event-count>
    </row>
  </node>
</trace-query-result>"#;
        let parsed = parse_single_row_table(xml).unwrap();
        assert_eq!(parsed.get("cpu-total").unwrap(), "42");
        assert_eq!(parsed.get("idle-wakeups").unwrap(), "7");
    }
}

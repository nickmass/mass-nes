use std::collections::{HashMap, HashSet};

use anyhow::{Result, anyhow, bail};
use xshell::{Shell, cmd};

fn main() -> Result<()> {
    let mut jobs = HashMap::new();
    for job in all_jobs() {
        if let Some(old_job) = jobs.insert(job.name(), job) {
            bail!("Duplicate Job name configured: '{}'", old_job.name());
        }
    }

    let mut args = std::env::args();
    let _ = args.next();
    let Some(task) = args.next() else {
        println!("Available tasks:");
        for job in all_jobs() {
            println!("\t{}", job.name());
        }
        bail!("No task specified")
    };

    let mut completed_jobs = HashSet::new();

    let sh = Shell::new()?;
    let job = jobs
        .get(task.as_str())
        .ok_or(anyhow!("Job not found: {task}"))?;
    run_with_deps(&sh, job, &jobs, &mut completed_jobs)
}

fn run_with_deps<J: Job>(
    sh: &Shell,
    job: &J,
    all_jobs: &HashMap<&'static str, Box<dyn Job>>,
    completed_jobs: &mut HashSet<&'static str>,
) -> Result<()> {
    for dep in job.depends() {
        let job = all_jobs.get(dep).ok_or(anyhow!("Job not found: '{dep}'"))?;
        run_with_deps(sh, job, all_jobs, completed_jobs)?;
    }

    if completed_jobs.insert(job.name()) {
        println!("Running: {}", job.name());
        job.run(sh)?;
    }

    Ok(())
}

fn all_jobs() -> impl Iterator<Item = Box<dyn Job>> {
    [
        Box::new(CleanTask) as Box<dyn Job>,
        Box::new(DesktopTask),
        Box::new(DebuggerTask),
        Box::new(WebTask),
        Box::new(DebuggerWebTask),
        Box::new(ReleaseTask),
    ]
    .into_iter()
}

trait Job {
    fn name(&self) -> &'static str;
    fn depends(&self) -> &'static [&'static str] {
        &[]
    }

    fn run(&self, sh: &Shell) -> Result<()>;
}

impl Job for Box<dyn Job> {
    fn name(&self) -> &'static str {
        (**self).name()
    }

    fn depends(&self) -> &'static [&'static str] {
        (**self).depends()
    }

    fn run(&self, sh: &Shell) -> Result<()> {
        (**self).run(sh)
    }
}

struct CleanTask;

impl Job for CleanTask {
    fn name(&self) -> &'static str {
        "clean"
    }

    fn run(&self, sh: &Shell) -> Result<()> {
        cmd!(sh, "cargo clean").run()?;

        sh.remove_path("target/web")?;
        sh.remove_path("target/debugger")?;
        sh.remove_path("dist")?;

        Ok(())
    }
}

struct DesktopTask;

impl Job for DesktopTask {
    fn name(&self) -> &'static str {
        "desktop"
    }

    fn run(&self, sh: &Shell) -> Result<()> {
        cmd!(sh, "cargo build --release -p desktop").run()?;
        cmd!(sh, "strip target/release/desktop").run()?;

        Ok(())
    }
}

struct DebuggerTask;

impl Job for DebuggerTask {
    fn name(&self) -> &'static str {
        "debugger"
    }

    fn run(&self, sh: &Shell) -> Result<()> {
        cmd!(sh, "cargo build --release -p debugger").run()?;
        cmd!(sh, "strip target/release/debugger").run()?;

        Ok(())
    }
}

struct WebTask;

impl Job for WebTask {
    fn name(&self) -> &'static str {
        "web"
    }

    fn run(&self, sh: &Shell) -> Result<()> {
        let _dir = sh.push_dir("crates/web");
        sh.create_dir("target/web")?;

        for f in sh.read_dir("static")? {
            sh.copy_file(f, "target/web")?;
        }

        cmd!(sh, "rustup run nightly wasm-pack build --release --weak-refs --reference-types --target web --no-typescript --no-pack -d target/web/pkg").run()?;
        sh.remove_path("target/web/pkg/.gitignore")?;
        cmd!(sh, "tar -C target -zcf target/web.tar.gz web").run()?;
        println!("Package created: crates/web/target/web.tar.gz");

        Ok(())
    }
}

struct DebuggerWebTask;

impl Job for DebuggerWebTask {
    fn name(&self) -> &'static str {
        "debugger-web"
    }

    fn run(&self, sh: &Shell) -> Result<()> {
        let _dir = sh.push_dir("crates/debugger");
        sh.create_dir("target/debugger")?;

        for f in sh.read_dir("static")? {
            sh.copy_file(f, "target/debugger")?;
        }

        cmd!(sh, "rustup run nightly wasm-pack build --release --weak-refs --reference-types --target web --no-typescript --no-pack -d target/debugger/pkg").run()?;
        sh.remove_path("target/debugger/pkg/.gitignore")?;
        cmd!(sh, "tar -C target -zcf target/debugger.tar.gz debugger").run()?;
        println!("Package created: crates/debugger/target/debugger.tar.gz");

        Ok(())
    }
}

struct ReleaseTask;

impl Job for ReleaseTask {
    fn name(&self) -> &'static str {
        "release"
    }

    fn depends(&self) -> &'static [&'static str] {
        &["clean", "web", "debugger-web", "desktop", "debugger"]
    }

    fn run(&self, sh: &Shell) -> Result<()> {
        let target = "linux-x86_64";

        sh.create_dir("dist")?;
        sh.copy_file("crates/web/target/web.tar.gz", "dist")?;
        sh.copy_file("crates/debugger/target/debugger.tar.gz", "dist")?;
        sh.copy_file("target/release/desktop", format!("dist/desktop-{target}"))?;
        sh.copy_file("target/release/debugger", format!("dist/debugger-{target}"))?;
        Ok(())
    }
}
